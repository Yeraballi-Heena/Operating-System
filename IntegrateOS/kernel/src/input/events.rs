//! Unified Input Event Architecture
//!
//! Provides abstract input representations for keyboard and pointing devices:
//! - `KeyCode`: Comprehensive scan code independent key identifiers
//! - `KeyAction`: Press, Release, and Repeat transitions
//! - `Modifiers`: Bitmask state for Shift, Ctrl, Alt, Super, CapsLock, NumLock
//! - `KeyEvent`: Rich keyboard event with key code, action, modifiers, and optional ASCII
//! - `MouseButton`: Left, Right, Middle button identifiers
//! - `MouseButtons`: Held button bitmask
//! - `MouseEvent`: Combined mouse delta, absolute position, button state, and scroll
//! - `InputEvent`: Unified event envelope with monotonic timestamp

/// Hardware-independent keyboard key identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum KeyCode {
    Unknown = 0,

    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers (Top Row)
    Num0, Num1, Num2, Num3, Num4,
    Num5, Num6, Num7, Num8, Num9,

    // Function Keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Control & Formatting
    Escape,
    Enter,
    Backspace,
    Tab,
    Space,
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Period,
    Slash,

    // Modifiers
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
    LeftSuper,
    RightSuper,

    // Locks
    CapsLock,
    ScrollLock,
    NumLock,

    // Navigation & Editing
    PrintScreen,
    Pause,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,

    // Keypad
    Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7, Kp8, Kp9,
    KpPlus, KpMinus, KpMultiply, KpDivide, KpEnter, KpDot,
}

/// Action state for keyboard keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Press,
    Release,
    Repeat,
}

/// Active modifier states associated with an input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers(pub u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1 << 0);
    pub const CTRL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);
    pub const SUPER: Self = Self(1 << 3);
    pub const CAPS_LOCK: Self = Self(1 << 4);
    pub const NUM_LOCK: Self = Self(1 << 5);
    pub const SCROLL_LOCK: Self = Self(1 << 6);

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub const fn is_shift(self) -> bool {
        (self.0 & Self::SHIFT.0) != 0
    }

    #[inline(always)]
    pub const fn is_ctrl(self) -> bool {
        (self.0 & Self::CTRL.0) != 0
    }

    #[inline(always)]
    pub const fn is_alt(self) -> bool {
        (self.0 & Self::ALT.0) != 0
    }

    #[inline(always)]
    pub const fn is_super(self) -> bool {
        (self.0 & Self::SUPER.0) != 0
    }

    #[inline(always)]
    pub const fn is_caps(self) -> bool {
        (self.0 & Self::CAPS_LOCK.0) != 0
    }
}

/// A structured keyboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub action: KeyAction,
    pub modifiers: Modifiers,
    pub ascii: Option<u8>,
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// Mouse button transition action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonAction {
    Press,
    Release,
}

/// Bitmask of currently held mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MouseButtons(pub u8);

impl MouseButtons {
    pub const NONE: Self = Self(0);
    pub const LEFT: Self = Self(1 << 0);
    pub const RIGHT: Self = Self(1 << 1);
    pub const MIDDLE: Self = Self(1 << 2);

    #[inline(always)]
    pub const fn is_left(self) -> bool {
        (self.0 & Self::LEFT.0) != 0
    }

    #[inline(always)]
    pub const fn is_right(self) -> bool {
        (self.0 & Self::RIGHT.0) != 0
    }

    #[inline(always)]
    pub const fn is_middle(self) -> bool {
        (self.0 & Self::MIDDLE.0) != 0
    }
}

/// A structured mouse state event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub position: (i32, i32),
    pub delta: (i32, i32),
    pub buttons: MouseButtons,
    pub scroll: i32,
}

/// Payload of a unified input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEventKind {
    Key(KeyEvent),
    Mouse(MouseEvent),
    MouseMove { position: (i32, i32), delta: (i32, i32) },
    MouseButton { button: MouseButton, action: MouseButtonAction, position: (i32, i32) },
    MouseScroll { delta: i32, position: (i32, i32) },
}

/// Unified input event with system timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub kind: InputEventKind,
    pub timestamp: u64,
}

impl InputEvent {
    pub fn new(kind: InputEventKind, timestamp: u64) -> Self {
        Self { kind, timestamp }
    }
}
