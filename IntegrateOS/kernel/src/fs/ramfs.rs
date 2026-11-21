//! In-Memory Tree Filesystem (RamFs)
//!
//! Provides memory-backed file (`RamFile`) and directory (`RamDirectory`) nodes
//! implementing the `VfsNode` trait with shared thread-safe inner state.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::fs::vfs::{DirEntry, Metadata, NodeType, VfsNode};
use crate::sync::IrqSpinlock;

static NEXT_INODE: AtomicU64 = AtomicU64::new(100);

fn allocate_inode() -> u64 {
    NEXT_INODE.fetch_add(1, Ordering::Relaxed)
}

/// Shared interior state of a `RamFile`.
pub struct RamFileInner {
    pub data: Vec<u8>,
}

/// A dynamic memory-backed file node sharing interior state across clones and open handles.
#[derive(Clone)]
pub struct RamFile {
    pub name: String,
    pub inode: u64,
    inner: Arc<IrqSpinlock<RamFileInner>>,
}

impl RamFile {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            inode: allocate_inode(),
            inner: Arc::new(IrqSpinlock::new(RamFileInner {
                data: Vec::new(),
            })),
        }
    }

    pub fn with_data(name: &str, initial_data: &[u8]) -> Self {
        Self {
            name: String::from(name),
            inode: allocate_inode(),
            inner: Arc::new(IrqSpinlock::new(RamFileInner {
                data: initial_data.to_vec(),
            })),
        }
    }
}

impl VfsNode for RamFile {
    fn name(&self) -> &str {
        &self.name
    }

    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn metadata(&self) -> Metadata {
        let size = self.inner.lock().data.len() as u64;
        Metadata {
            size,
            node_type: NodeType::File,
            inode: self.inode,
            mode: 0o644,
        }
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, &'static str> {
        let inner = self.inner.lock();
        let offset = offset as usize;
        if offset >= inner.data.len() {
            return Ok(0); // EOF
        }
        let available = inner.data.len() - offset;
        let to_read = core::cmp::min(buf.len(), available);
        buf[..to_read].copy_from_slice(&inner.data[offset..offset + to_read]);
        Ok(to_read)
    }

    fn write(&mut self, offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        let mut inner = self.inner.lock();
        let offset = offset as usize;
        let end = offset + buf.len();
        if end > inner.data.len() {
            inner.data.resize(end, 0);
        }
        inner.data[offset..end].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn lookup(&self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot lookup on a file")
    }

    fn create(&mut self, _name: &str, _node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot create inside a file")
    }

    fn mkdir(&mut self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot mkdir inside a file")
    }

    fn remove(&mut self, _name: &str) -> Result<(), &'static str> {
        Err("Cannot remove inside a file")
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        Err("Cannot readdir on a file")
    }

    fn truncate(&mut self, size: u64) -> Result<(), &'static str> {
        self.inner.lock().data.resize(size as usize, 0);
        Ok(())
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }
}

/// Shared interior state of a `RamDirectory`.
pub struct RamDirectoryInner {
    pub children: Vec<Box<dyn VfsNode>>,
}

/// A dynamic memory-backed directory node sharing interior state across clones and open handles.
#[derive(Clone)]
pub struct RamDirectory {
    pub name: String,
    pub inode: u64,
    inner: Arc<IrqSpinlock<RamDirectoryInner>>,
}

impl RamDirectory {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            inode: allocate_inode(),
            inner: Arc::new(IrqSpinlock::new(RamDirectoryInner {
                children: Vec::new(),
            })),
        }
    }

    /// Adds an existing VFS node as a child of this directory.
    pub fn add_child(&self, node: Box<dyn VfsNode>) {
        let mut inner = self.inner.lock();
        inner.children.retain(|c| c.name() != node.name());
        inner.children.push(node);
    }
}

impl VfsNode for RamDirectory {
    fn name(&self) -> &str {
        &self.name
    }

    fn node_type(&self) -> NodeType {
        NodeType::Directory
    }

    fn metadata(&self) -> Metadata {
        let size = self.inner.lock().children.len() as u64;
        Metadata {
            size,
            node_type: NodeType::Directory,
            inode: self.inode,
            mode: 0o755,
        }
    }

    fn read(&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("Cannot read directly from a directory; use readdir")
    }

    fn write(&mut self, _offset: u64, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("Cannot write directly to a directory; use create or mkdir")
    }

    fn lookup(&self, name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        if name == "." {
            return Ok(self.clone_node());
        }
        let inner = self.inner.lock();
        for child in &inner.children {
            if child.name() == name {
                return Ok(child.clone_node());
            }
        }
        Err("No such file or directory (ENOENT)")
    }

    fn create(&mut self, name: &str, node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        let mut inner = self.inner.lock();
        if inner.children.iter().any(|c| c.name() == name) {
            return Err("File already exists (EEXIST)");
        }
        let new_node: Box<dyn VfsNode> = match node_type {
            NodeType::File => Box::new(RamFile::new(name)),
            NodeType::Directory => Box::new(RamDirectory::new(name)),
            _ => return Err("Unsupported node type for generic create"),
        };
        let ret = new_node.clone_node();
        inner.children.push(new_node);
        Ok(ret)
    }

    fn mkdir(&mut self, name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        self.create(name, NodeType::Directory)
    }

    fn remove(&mut self, name: &str) -> Result<(), &'static str> {
        let mut inner = self.inner.lock();
        let initial_len = inner.children.len();
        inner.children.retain(|c| c.name() != name);
        if inner.children.len() < initial_len {
            Ok(())
        } else {
            Err("No such file or directory (ENOENT)")
        }
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        let inner = self.inner.lock();
        let mut entries = Vec::new();
        // Add "." and ".."
        entries.push(DirEntry {
            name: String::from("."),
            inode: self.inode,
            node_type: NodeType::Directory,
            size: inner.children.len() as u64,
        });
        entries.push(DirEntry {
            name: String::from(".."),
            inode: 1, // Root/parent reference
            node_type: NodeType::Directory,
            size: 0,
        });

        for child in &inner.children {
            let meta = child.metadata();
            entries.push(DirEntry {
                name: String::from(child.name()),
                inode: meta.inode,
                node_type: meta.node_type,
                size: meta.size,
            });
        }
        Ok(entries)
    }

    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Err("Cannot truncate a directory")
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }

    fn add_child_node(&mut self, node: Box<dyn VfsNode>) -> Result<(), &'static str> {
        self.add_child(node);
        Ok(())
    }
}
