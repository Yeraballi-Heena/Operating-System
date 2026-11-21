//! File Descriptor Table & Active File Handles
//!
//! Manages open file handles, read/write offsets, and standard POSIX descriptors (0, 1, 2).

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::fs::devfs::DevConsole;
use crate::fs::vfs::{OpenFlags, SeekFrom, VfsNode};

/// An active open file handle.
pub struct FileHandle {
    pub node: Box<dyn VfsNode>,
    pub offset: u64,
    pub flags: OpenFlags,
}

impl FileHandle {
    pub fn new(node: Box<dyn VfsNode>, flags: OpenFlags) -> Self {
        Self {
            node,
            offset: 0,
            flags,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let bytes = self.node.read(self.offset, buf)?;
        self.offset += bytes as u64;
        Ok(bytes)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if self.flags.contains(OpenFlags::APPEND) {
            self.offset = self.node.metadata().size;
        }
        let bytes = self.node.write(self.offset, buf)?;
        self.offset += bytes as u64;
        Ok(bytes)
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, &'static str> {
        let file_size = self.node.metadata().size as i64;
        let new_offset = match pos {
            SeekFrom::Start(off) => off as i64,
            SeekFrom::Current(off) => (self.offset as i64) + off,
            SeekFrom::End(off) => file_size + off,
        };

        if new_offset < 0 {
            return Err("Invalid seek offset (negative)");
        }
        self.offset = new_offset as u64;
        Ok(self.offset)
    }
}

/// System-wide or process-level file descriptor table.
pub struct FileTable {
    handles: Vec<Option<FileHandle>>,
}

impl FileTable {
    pub fn new() -> Self {
        let mut table = Self {
            handles: Vec::new(),
        };

        // FD 0: stdin (/dev/console, read)
        table.handles.push(Some(FileHandle::new(
            Box::new(DevConsole),
            OpenFlags::READ,
        )));

        // FD 1: stdout (/dev/console, write)
        table.handles.push(Some(FileHandle::new(
            Box::new(DevConsole),
            OpenFlags::WRITE,
        )));

        // FD 2: stderr (/dev/console, write)
        table.handles.push(Some(FileHandle::new(
            Box::new(DevConsole),
            OpenFlags::WRITE,
        )));

        table
    }

    /// Allocates the next lowest available file descriptor.
    pub fn allocate(&mut self, handle: FileHandle) -> usize {
        for (fd, slot) in self.handles.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(handle);
                return fd;
            }
        }
        self.handles.push(Some(handle));
        self.handles.len() - 1
    }

    /// Returns a reference to the handle at `fd`.
    pub fn get(&self, fd: usize) -> Option<&FileHandle> {
        self.handles.get(fd).and_then(|slot| slot.as_ref())
    }

    /// Returns a mutable reference to the handle at `fd`.
    pub fn get_mut(&mut self, fd: usize) -> Option<&mut FileHandle> {
        self.handles.get_mut(fd).and_then(|slot| slot.as_mut())
    }

    /// Closes an open file descriptor.
    pub fn close(&mut self, fd: usize) -> Result<(), &'static str> {
        if fd >= self.handles.len() || self.handles[fd].is_none() {
            return Err("Bad file descriptor (EBADF)");
        }
        self.handles[fd] = None;
        Ok(())
    }
}
