//! Virtual Filesystem (VFS) Core Types & Node Trait
//!
//! Provides abstract filesystem nodes (`VfsNode`), node types, open flags,
//! metadata, directory entries, and seek operations.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// Classification of a VFS node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
    CharDevice,
    BlockDevice,
    Symlink,
}

impl NodeType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Directory => "Directory",
            Self::CharDevice => "CharDevice",
            Self::BlockDevice => "BlockDevice",
            Self::Symlink => "Symlink",
        }
    }
}

/// Bitflags governing file opening behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenFlags(pub u32);

impl OpenFlags {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const CREATE: Self = Self(1 << 2);
    pub const APPEND: Self = Self(1 << 3);
    pub const TRUNCATE: Self = Self(1 << 4);

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for OpenFlags {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for OpenFlags {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Positioning modes for file seek operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    Current(i64),
    End(i64),
}

/// Metadata attributes associated with a VFS node.
#[derive(Debug, Clone)]
pub struct Metadata {
    pub size: u64,
    pub node_type: NodeType,
    pub inode: u64,
    pub mode: u32,
}

/// A directory entry describing a child item in a directory listing.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub inode: u64,
    pub node_type: NodeType,
    pub size: u64,
}

/// Abstract Virtual Filesystem Node interface.
pub trait VfsNode: Send + Sync {
    /// Name of this file or directory component.
    fn name(&self) -> &str;

    /// Node category (File, Directory, Device, etc.).
    fn node_type(&self) -> NodeType;

    /// File size and metadata attributes.
    fn metadata(&self) -> Metadata;

    /// Reads up to `buf.len()` bytes starting at `offset`.
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, &'static str>;

    /// Writes `buf` bytes starting at `offset`.
    fn write(&mut self, offset: u64, buf: &[u8]) -> Result<usize, &'static str>;

    /// Looks up a child node by name inside a directory.
    fn lookup(&self, name: &str) -> Result<Box<dyn VfsNode>, &'static str>;

    /// Creates a new child file or node in this directory.
    fn create(&mut self, name: &str, node_type: NodeType) -> Result<Box<dyn VfsNode>, &'static str>;

    /// Creates a new child subdirectory.
    fn mkdir(&mut self, name: &str) -> Result<Box<dyn VfsNode>, &'static str>;

    /// Removes a child node by name from this directory.
    fn remove(&mut self, name: &str) -> Result<(), &'static str>;

    /// Lists all entries contained within this directory.
    fn readdir(&self) -> Result<Vec<DirEntry>, &'static str>;

    /// Truncates or extends file to `size` bytes.
    fn truncate(&mut self, size: u64) -> Result<(), &'static str>;

    /// Clones this node as a boxed trait object.
    fn clone_node(&self) -> Box<dyn VfsNode>;

    /// Adds a child node directly into a directory node.
    fn add_child_node(&mut self, _node: Box<dyn VfsNode>) -> Result<(), &'static str> {
        Err("Cannot add child node to non-directory")
    }
}
