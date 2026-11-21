//! Integrate OS — Virtual Filesystem (VFS) Architecture
//!
//! Provides:
//! - Abstract VFS node definitions (`VfsNode`, `NodeType`, `Metadata`, `DirEntry`)
//! - Canonical path parsing and normalization (`path.rs`)
//! - In-memory tree filesystem (`RamFs`, `ramfs.rs`)
//! - Device node filesystem (`DevFs`, `devfs.rs`) mounted at `/dev`
//! - Process / kernel file descriptor table (`FileTable`, `file_table.rs`)
//! - Central thread-safe VFS manager (`VFS`)
//! - Phase 15 verification test suite (`tests.rs`)

pub mod devfs;
pub mod file_table;
pub mod path;
pub mod ramfs;
pub mod tests;
pub mod vfs;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::fs::devfs::{DevConsole, DevHda, DevNull, DevZero};
use crate::fs::file_table::{FileHandle, FileTable};
use crate::fs::path::{normalize_path, split_path};
use crate::fs::ramfs::{RamDirectory, RamFile};
pub use crate::fs::vfs::{DirEntry, Metadata, NodeType, OpenFlags, SeekFrom, VfsNode};
use crate::sync::IrqSpinlock;
use crate::klog_ok;

/// Central Virtual Filesystem Manager.
pub struct VfsManager {
    root: Box<dyn VfsNode>,
    file_table: FileTable,
}

impl VfsManager {
    pub fn new() -> Self {
        Self {
            root: Box::new(RamDirectory::new("")),
            file_table: FileTable::new(),
        }
    }

    pub fn with_root(root: Box<dyn VfsNode>) -> Self {
        Self {
            root,
            file_table: FileTable::new(),
        }
    }

    /// Resolves an absolute or relative path to its corresponding `VfsNode`.
    pub fn resolve(&self, path: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        let norm = normalize_path(path);
        if norm == "/" {
            return Ok(self.root.clone_node());
        }

        let mut current = self.root.clone_node();
        for seg in norm.split('/').filter(|s| !s.is_empty()) {
            current = current.lookup(seg)?;
        }
        Ok(current)
    }

    /// Opens a file at `path` with `flags` and returns a file descriptor index.
    pub fn open(&mut self, path: &str, flags: OpenFlags) -> Result<usize, &'static str> {
        let norm = normalize_path(path);

        // Attempt direct resolution first
        let node_res = self.resolve(&norm);

        let node = match node_res {
            Ok(mut n) => {
                if flags.contains(OpenFlags::TRUNCATE) && n.node_type() == NodeType::File {
                    n.truncate(0)?;
                }
                n
            }
            Err(_) => {
                if flags.contains(OpenFlags::CREATE) {
                    let (parent_path, child_name) = split_path(&norm);
                    let mut parent = self.resolve(parent_path)?;
                    parent.create(child_name, NodeType::File)?
                } else {
                    return Err("No such file or directory (ENOENT)");
                }
            }
        };

        let handle = FileHandle::new(node, flags);
        Ok(self.file_table.allocate(handle))
    }

    /// Closes an open file descriptor.
    pub fn close(&mut self, fd: usize) -> Result<(), &'static str> {
        self.file_table.close(fd)
    }

    /// Reads up to `buf.len()` bytes from file descriptor `fd`.
    pub fn read(&mut self, fd: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
        let handle = self.file_table.get_mut(fd).ok_or("Bad file descriptor (EBADF)")?;
        handle.read(buf)
    }

    /// Writes `buf` bytes to file descriptor `fd`.
    pub fn write(&mut self, fd: usize, buf: &[u8]) -> Result<usize, &'static str> {
        let handle = self.file_table.get_mut(fd).ok_or("Bad file descriptor (EBADF)")?;
        handle.write(buf)
    }

    /// Updates read/write offset of file descriptor `fd`.
    pub fn seek(&mut self, fd: usize, pos: SeekFrom) -> Result<u64, &'static str> {
        let handle = self.file_table.get_mut(fd).ok_or("Bad file descriptor (EBADF)")?;
        handle.seek(pos)
    }

    /// Creates a directory at `path`.
    pub fn mkdir(&mut self, path: &str) -> Result<(), &'static str> {
        let norm = normalize_path(path);
        let (parent_path, child_name) = split_path(&norm);
        let mut parent = self.resolve(parent_path)?;
        parent.mkdir(child_name)?;
        Ok(())
    }

    /// Removes a file or empty directory at `path`.
    pub fn remove(&mut self, path: &str) -> Result<(), &'static str> {
        let norm = normalize_path(path);
        let (parent_path, child_name) = split_path(&norm);
        let mut parent = self.resolve(parent_path)?;
        parent.remove(child_name)
    }

    /// Lists directory entries at `path`.
    pub fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, &'static str> {
        let node = self.resolve(path)?;
        node.readdir()
    }

    /// Returns metadata attributes for `path`.
    pub fn stat(&self, path: &str) -> Result<Metadata, &'static str> {
        let node = self.resolve(path)?;
        Ok(node.metadata())
    }

    /// Mounts a custom `VfsNode` directly into the directory at `parent_path`.
    pub fn mount_node(&mut self, parent_path: &str, node: Box<dyn VfsNode>) -> Result<(), &'static str> {
        let mut parent = self.resolve(parent_path)?;
        parent.add_child_node(node)
    }
}

pub static VFS: IrqSpinlock<Option<VfsManager>> = IrqSpinlock::new(None);

/// Initializes the Virtual Filesystem and mounts standard directory hierarchy.
pub fn init() {
    let root_dir = RamDirectory::new("");

    // 1. Create standard root directories
    let dev_dir = Box::new(RamDirectory::new("dev"));
    dev_dir.add_child(Box::new(DevNull));
    dev_dir.add_child(Box::new(DevZero));
    dev_dir.add_child(Box::new(DevConsole));
    dev_dir.add_child(Box::new(DevHda));

    let etc_dir = Box::new(RamDirectory::new("etc"));
    etc_dir.add_child(Box::new(RamFile::with_data(
        "os-release",
        b"NAME=\"Integrate OS\"\nVERSION=\"0.1.0\"\nID=integrate-os\nPRETTY_NAME=\"Integrate OS (64-Bit Pure Rust)\"\nARCHITECTURE=\"x86_64\"\n",
    )));
    etc_dir.add_child(Box::new(RamFile::with_data(
        "hostname",
        b"integrate-os\n",
    )));

    // Attach to root
    root_dir.add_child(dev_dir);
    root_dir.add_child(etc_dir);
    root_dir.add_child(Box::new(RamDirectory::new("bin")));
    root_dir.add_child(Box::new(RamDirectory::new("home")));
    root_dir.add_child(Box::new(RamDirectory::new("tmp")));
    root_dir.add_child(Box::new(RamDirectory::new("proc")));

    let vfs = VfsManager::with_root(Box::new(root_dir));
    *VFS.lock() = Some(vfs);
    klog_ok!("Virtual Filesystem (VFS) Initialized: Root RamFs mounted, /dev active");
}

/// Opens a file in the global VFS.
pub fn open(path: &str, flags: OpenFlags) -> Result<usize, &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.open(path, flags)
}

/// Closes an open file descriptor in the global VFS.
pub fn close(fd: usize) -> Result<(), &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.close(fd)
}

/// Reads from an open file descriptor in the global VFS.
pub fn read(fd: usize, buf: &mut [u8]) -> Result<usize, &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.read(fd, buf)
}

/// Writes to an open file descriptor in the global VFS.
pub fn write(fd: usize, buf: &[u8]) -> Result<usize, &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.write(fd, buf)
}

/// Repositions offset in an open file descriptor in the global VFS.
pub fn seek(fd: usize, pos: SeekFrom) -> Result<u64, &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.seek(fd, pos)
}

/// Creates a directory in the global VFS.
pub fn mkdir(path: &str) -> Result<(), &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.mkdir(path)
}

/// Removes a file or directory in the global VFS.
pub fn remove(path: &str) -> Result<(), &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.remove(path)
}

/// Lists entries in a directory in the global VFS.
pub fn readdir(path: &str) -> Result<Vec<DirEntry>, &'static str> {
    let vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_ref().ok_or("VFS not initialized")?;
    vfs.readdir(path)
}

/// Queries metadata for a path in the global VFS.
pub fn stat(path: &str) -> Result<Metadata, &'static str> {
    let vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_ref().ok_or("VFS not initialized")?;
    vfs.stat(path)
}

/// Reads an entire file into a byte vector.
pub fn read_file(path: &str) -> Result<Vec<u8>, &'static str> {
    let fd = open(path, OpenFlags::READ)?;
    let mut content = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let bytes = read(fd, &mut chunk)?;
        if bytes == 0 {
            break;
        }
        content.extend_from_slice(&chunk[..bytes]);
    }
    close(fd)?;
    Ok(content)
}

/// Writes an entire buffer to a file (creating or truncating).
pub fn write_file(path: &str, data: &[u8]) -> Result<(), &'static str> {
    let fd = open(path, OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE)?;
    write(fd, data)?;
    close(fd)?;
    Ok(())
}

/// Mounts an arbitrary `VfsNode` trait object into `parent_path`.
pub fn mount_node(parent_path: &str, node: Box<dyn VfsNode>) -> Result<(), &'static str> {
    let mut vfs_lock = VFS.lock();
    let vfs = vfs_lock.as_mut().ok_or("VFS not initialized")?;
    vfs.mount_node(parent_path, node)
}
