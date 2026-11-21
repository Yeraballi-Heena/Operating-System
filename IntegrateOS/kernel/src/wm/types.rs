//! Window Manager Core Types & Flags
//!
//! Provides fundamental identifiers, states, flags, hit-test categories, and edge definitions.

/// Unique monotonic window identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(pub u32);

impl WindowId {
    pub const NONE: Self = Self(0);
}

/// Visual presentation and lifecycle states of a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Hidden,
}

/// Bitflags specifying capabilities and style attributes of a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFlags(pub u32);

impl WindowFlags {
    pub const NONE: Self = Self(0);
    pub const RESIZABLE: Self = Self(1 << 0);
    pub const MOVABLE: Self = Self(1 << 1);
    pub const CLOSABLE: Self = Self(1 << 2);
    pub const MINIMIZABLE: Self = Self(1 << 3);
    pub const MAXIMIZABLE: Self = Self(1 << 4);
    pub const ALWAYS_ON_TOP: Self = Self(1 << 5);
    pub const BORDERLESS: Self = Self(1 << 6);
    pub const MODAL: Self = Self(1 << 7);

    /// Standard window capabilities (movable, resizable, closable, minimizable, maximizable).
    pub const DEFAULT: Self = Self(
        Self::RESIZABLE.0
            | Self::MOVABLE.0
            | Self::CLOSABLE.0
            | Self::MINIMIZABLE.0
            | Self::MAXIMIZABLE.0,
    );

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for WindowFlags {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for WindowFlags {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Edge and corner classifications for window resizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Region classification returned by window hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestResult {
    Nowhere,
    ClientArea,
    TitleBar,
    CloseButton,
    MaximizeButton,
    MinimizeButton,
    ResizeBorder(ResizeEdge),
}
