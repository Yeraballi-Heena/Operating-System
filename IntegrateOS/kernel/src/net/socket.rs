//! Integrate OS — Network Socket Subsystem
//!
//! Provides a unified socket descriptor table and POSIX-like networking primitives:
//! `socket`, `bind`, `sendto`, `recvfrom`, and `close`.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::net::stack::NETWORK_STACK;
use crate::net::types::{NetError, SocketAddr};
use crate::sync::IrqSpinlock;

static NEXT_SOCKET_ID: AtomicU32 = AtomicU32::new(1);

/// Strongly-typed network socket descriptor.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketHandle(pub u32);

impl SocketHandle {
    pub fn next() -> Self {
        Self(NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Socket Classification.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SocketType {
    Udp,
    TcpStream,
    TcpListener,
}

/// Active Kernel Socket Descriptor.
pub struct Socket {
    pub handle: SocketHandle,
    pub sock_type: SocketType,
    pub local_port: Option<u16>,
    pub remote_addr: Option<SocketAddr>,
}

/// Socket Table Manager.
pub struct SocketTable {
    pub sockets: Vec<Socket>,
}

impl SocketTable {
    pub const fn new() -> Self {
        Self {
            sockets: Vec::new(),
        }
    }

    /// Allocates a new socket descriptor of the specified type.
    pub fn create(&mut self, sock_type: SocketType) -> SocketHandle {
        let handle = SocketHandle::next();
        self.sockets.push(Socket {
            handle,
            sock_type,
            local_port: None,
            remote_addr: None,
        });
        handle
    }

    /// Binds a socket to a local port.
    pub fn bind(&mut self, handle: SocketHandle, port: u16) -> Result<(), NetError> {
        let sock = self
            .sockets
            .iter_mut()
            .find(|s| s.handle == handle)
            .ok_or(NetError::SocketClosed)?;

        if sock.sock_type == SocketType::Udp {
            let mut stack = NETWORK_STACK.lock();
            stack.udp_sockets.bind(port)?;
        }

        sock.local_port = Some(port);
        Ok(())
    }

    /// Closes a socket and unbinds resources.
    pub fn close(&mut self, handle: SocketHandle) {
        if let Some(pos) = self.sockets.iter().position(|s| s.handle == handle) {
            let sock = self.sockets.remove(pos);
            if let Some(port) = sock.local_port {
                if sock.sock_type == SocketType::Udp {
                    let mut stack = NETWORK_STACK.lock();
                    stack.udp_sockets.close(port);
                }
            }
        }
    }
}

pub static SOCKET_TABLE: IrqSpinlock<SocketTable> = IrqSpinlock::new(SocketTable::new());

/// Creates a new socket descriptor.
pub fn socket(sock_type: SocketType) -> SocketHandle {
    SOCKET_TABLE.lock().create(sock_type)
}

/// Binds a socket to a local port.
pub fn bind(handle: SocketHandle, port: u16) -> Result<(), NetError> {
    SOCKET_TABLE.lock().bind(handle, port)
}

/// Sends data via UDP socket to destination address.
pub fn send_to(handle: SocketHandle, dst: SocketAddr, data: &[u8]) -> Result<(), NetError> {
    let port = {
        let table = SOCKET_TABLE.lock();
        let sock = table
            .sockets
            .iter()
            .find(|s| s.handle == handle)
            .ok_or(NetError::SocketClosed)?;
        sock.local_port.unwrap_or(49152) // Default ephemeral port
    };

    let mut stack = NETWORK_STACK.lock();
    stack.send_udp(port, dst, data)
}

/// Receives an incoming datagram on a bound socket.
pub fn recv_from(handle: SocketHandle, buf: &mut [u8]) -> Result<(usize, SocketAddr), NetError> {
    let port = {
        let table = SOCKET_TABLE.lock();
        let sock = table
            .sockets
            .iter()
            .find(|s| s.handle == handle)
            .ok_or(NetError::SocketClosed)?;
        sock.local_port.ok_or(NetError::NotConnected)?
    };

    let mut stack = NETWORK_STACK.lock();
    stack.poll_ingress();

    if let Some(datagram) = stack.udp_sockets.recv_from(port) {
        if buf.len() < datagram.payload.len() {
            return Err(NetError::BufferTooSmall);
        }
        buf[..datagram.payload.len()].copy_from_slice(&datagram.payload);
        Ok((datagram.payload.len(), datagram.src_addr))
    } else {
        Err(NetError::Timeout)
    }
}

/// Closes a socket descriptor.
pub fn close(handle: SocketHandle) {
    SOCKET_TABLE.lock().close(handle);
}
