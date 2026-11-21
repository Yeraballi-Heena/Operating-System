//! Special Device Nodes (DevFs)
//!
//! Provides character and block device nodes for `/dev`:
//! - `/dev/null`: Data sink (discards writes, immediate EOF on read)
//! - `/dev/zero`: Source of zeroes
//! - `/dev/console`: Terminal console I/O
//! - `/dev/hda`: Hardware block storage device

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::arch::x86_64::keyboard::KEYBOARD_QUEUE;
use crate::fs::vfs::{DirEntry, Metadata, NodeType, VfsNode};
use crate::kprint;

/// `/dev/null` device node.
#[derive(Clone)]
pub struct DevNull;

impl VfsNode for DevNull {
    fn name(&self) -> &str {
        "null"
    }

    fn node_type(&self) -> NodeType {
        NodeType::CharDevice
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            size: 0,
            node_type: NodeType::CharDevice,
            inode: 2,
            mode: 0o666,
        }
    }

    fn read(&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Ok(0) // Immediate EOF
    }

    fn write(&mut self, _offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len()) // Discard all input
    }

    fn lookup(&self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot lookup on device")
    }

    fn create(&mut self, _name: &str, _node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot create in device")
    }

    fn mkdir(&mut self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot mkdir in device")
    }

    fn remove(&mut self, _name: &str) -> Result<(), &'static str> {
        Err("Cannot remove from device")
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        Err("Cannot readdir on device")
    }

    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Ok(())
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }
}

/// `/dev/zero` device node.
#[derive(Clone)]
pub struct DevZero;

impl VfsNode for DevZero {
    fn name(&self) -> &str {
        "zero"
    }

    fn node_type(&self) -> NodeType {
        NodeType::CharDevice
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            size: 0,
            node_type: NodeType::CharDevice,
            inode: 3,
            mode: 0o666,
        }
    }

    fn read(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, &'static str> {
        buf.fill(0);
        Ok(buf.len())
    }

    fn write(&mut self, _offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len())
    }

    fn lookup(&self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot lookup on device")
    }

    fn create(&mut self, _name: &str, _node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot create in device")
    }

    fn mkdir(&mut self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot mkdir in device")
    }

    fn remove(&mut self, _name: &str) -> Result<(), &'static str> {
        Err("Cannot remove from device")
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        Err("Cannot readdir on device")
    }

    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Ok(())
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }
}

/// `/dev/console` device node.
#[derive(Clone)]
pub struct DevConsole;

impl VfsNode for DevConsole {
    fn name(&self) -> &str {
        "console"
    }

    fn node_type(&self) -> NodeType {
        NodeType::CharDevice
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            size: 0,
            node_type: NodeType::CharDevice,
            inode: 4,
            mode: 0o666,
        }
    }

    fn read(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, &'static str> {
        let mut read_bytes = 0;
        let mut queue = KEYBOARD_QUEUE.lock();
        while read_bytes < buf.len() {
            if let Some(ch) = queue.pop() {
                buf[read_bytes] = ch;
                read_bytes += 1;
            } else {
                break;
            }
        }
        Ok(read_bytes)
    }

    fn write(&mut self, _offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        if let Ok(s) = core::str::from_utf8(buf) {
            kprint!("{}", s);
        } else {
            for &b in buf {
                kprint!("{}", b as char);
            }
        }
        Ok(buf.len())
    }

    fn lookup(&self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot lookup on device")
    }

    fn create(&mut self, _name: &str, _node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot create in device")
    }

    fn mkdir(&mut self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot mkdir in device")
    }

    fn remove(&mut self, _name: &str) -> Result<(), &'static str> {
        Err("Cannot remove from device")
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        Err("Cannot readdir on device")
    }

    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Ok(())
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }
}

/// `/dev/hda` block storage device node.
#[derive(Clone)]
pub struct DevHda;

impl VfsNode for DevHda {
    fn name(&self) -> &str {
        "hda"
    }

    fn node_type(&self) -> NodeType {
        NodeType::BlockDevice
    }

    fn metadata(&self) -> Metadata {
        let total_sectors = crate::storage::total_blocks();
        Metadata {
            size: total_sectors * 512,
            node_type: NodeType::BlockDevice,
            inode: 5,
            mode: 0o660,
        }
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, &'static str> {
        if buf.len() < 512 {
            return Err("Read buffer must be at least 512 bytes for block device");
        }
        let sector = offset / 512;
        crate::storage::read_block(sector, buf)?;
        Ok(512)
    }

    fn write(&mut self, offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        if buf.len() < 512 {
            return Err("Write buffer must be at least 512 bytes for block device");
        }
        let sector = offset / 512;
        crate::storage::write_block(sector, buf)?;
        Ok(512)
    }

    fn lookup(&self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot lookup on device")
    }

    fn create(&mut self, _name: &str, _node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot create in device")
    }

    fn mkdir(&mut self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot mkdir in device")
    }

    fn remove(&mut self, _name: &str) -> Result<(), &'static str> {
        Err("Cannot remove from device")
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        Err("Cannot readdir on device")
    }

    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Err("Cannot truncate block device")
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }
}
