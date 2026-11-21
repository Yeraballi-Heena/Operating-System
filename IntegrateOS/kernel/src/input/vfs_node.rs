//! Input VFS Device Nodes (`/dev/input/mice` and `/dev/input/event0`)
//!
//! Provides character device nodes exposing mouse packets and structured input
//! events through standard VFS interfaces.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::fs::vfs::{DirEntry, Metadata, NodeType, VfsNode};
use crate::input::events::InputEventKind;
use crate::input::mouse::MOUSE;
use crate::input::pop_event;

/// `/dev/input/mice` device node.
///
/// Exposes streaming standard 3-byte PS/2 mouse packets `[flags, dx, dy]`.
#[derive(Clone)]
pub struct DevMice;

impl VfsNode for DevMice {
    fn name(&self) -> &str {
        "mice"
    }

    fn node_type(&self) -> NodeType {
        NodeType::CharDevice
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            size: 0,
            node_type: NodeType::CharDevice,
            inode: 60,
            mode: 0o660,
        }
    }

    fn read(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, &'static str> {
        if buf.len() < 3 {
            return Ok(0);
        }

        let mouse = MOUSE.lock();
        let packet = mouse.last_raw_packet();
        buf[0] = packet[0];
        buf[1] = packet[1];
        buf[2] = packet[2];
        Ok(3)
    }

    fn write(&mut self, _offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len()) // Discard writes
    }

    fn lookup(&self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot lookup on character device")
    }

    fn create(&mut self, _name: &str, _node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot create inside character device")
    }

    fn mkdir(&mut self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot mkdir inside character device")
    }

    fn remove(&mut self, _name: &str) -> Result<(), &'static str> {
        Err("Cannot remove from character device")
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        Err("Cannot readdir on character device")
    }

    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Ok(())
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }
}

/// `/dev/input/event0` device node.
///
/// Exposes structured 16-byte input event records:
/// - `offset 0..8`: monotonic timestamp (`u64`)
/// - `offset 8..10`: event type (`u16`)
/// - `offset 10..12`: event code (`u16`)
/// - `offset 12..16`: event value (`i32`)
#[derive(Clone)]
pub struct DevEvent0;

impl VfsNode for DevEvent0 {
    fn name(&self) -> &str {
        "event0"
    }

    fn node_type(&self) -> NodeType {
        NodeType::CharDevice
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            size: 0,
            node_type: NodeType::CharDevice,
            inode: 61,
            mode: 0o660,
        }
    }

    fn read(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, &'static str> {
        if buf.len() < 16 {
            return Ok(0);
        }

        let Some(ev) = pop_event() else {
            return Ok(0);
        };

        // Serialize into 16-byte record
        let (ev_type, ev_code, ev_val) = match ev.kind {
            InputEventKind::Key(k) => (1u16, k.code as u16, k.action as i32),
            InputEventKind::MouseMove { delta, .. } => (2u16, delta.0 as u16, delta.1),
            InputEventKind::MouseButton { button, action, .. } => {
                let btn_code = match button {
                    crate::input::events::MouseButton::Left => 0u16,
                    crate::input::events::MouseButton::Right => 1u16,
                    crate::input::events::MouseButton::Middle => 2u16,
                    crate::input::events::MouseButton::Other(c) => c as u16,
                };
                (3u16, btn_code, action as i32)
            }
            InputEventKind::MouseScroll { delta, .. } => (4u16, 0u16, delta),
            InputEventKind::Mouse(m) => (5u16, m.buttons.0 as u16, m.position.0),
        };

        let ts_bytes = ev.timestamp.to_ne_bytes();
        let type_bytes = ev_type.to_ne_bytes();
        let code_bytes = ev_code.to_ne_bytes();
        let val_bytes = ev_val.to_ne_bytes();

        buf[0..8].copy_from_slice(&ts_bytes);
        buf[8..10].copy_from_slice(&type_bytes);
        buf[10..12].copy_from_slice(&code_bytes);
        buf[12..16].copy_from_slice(&val_bytes);

        Ok(16)
    }

    fn write(&mut self, _offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len())
    }

    fn lookup(&self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot lookup on character device")
    }

    fn create(&mut self, _name: &str, _node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot create inside character device")
    }

    fn mkdir(&mut self, _name: &str) -> Result<Box<dyn VfsNode>, &'static str> {
        Err("Cannot mkdir inside character device")
    }

    fn remove(&mut self, _name: &str) -> Result<(), &'static str> {
        Err("Cannot remove from character device")
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str> {
        Err("Cannot readdir on character device")
    }

    fn truncate(&mut self, _size: u64) -> Result<(), &'static str> {
        Ok(())
    }

    fn clone_node(&self) -> Box<dyn VfsNode> {
        Box::new(self.clone())
    }
}
