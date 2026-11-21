//! Integrate OS — System Clipboard Service
//!
//! Provides thread-safe inter-application text copy/paste buffer.

use alloc::string::String;
use crate::sync::IrqSpinlock;

pub const MAX_CLIPBOARD_BYTES: usize = 4096;

/// System-wide shared clipboard buffer.
#[derive(Debug, Default)]
pub struct Clipboard {
    text: Option<String>,
}

impl Clipboard {
    pub const fn new() -> Self {
        Self { text: None }
    }

    /// Stores a text string in the clipboard (truncated to MAX_CLIPBOARD_BYTES).
    pub fn set_text(&mut self, text: &str) {
        let limit = text.len().min(MAX_CLIPBOARD_BYTES);
        self.text = Some(String::from(&text[..limit]));
    }

    /// Retrieves a copy of the current clipboard text.
    pub fn get_text(&self) -> Option<String> {
        self.text.clone()
    }

    /// Returns `true` if text is currently stored in the clipboard.
    pub fn has_text(&self) -> bool {
        self.text.is_some()
    }

    /// Clears any content stored in the clipboard.
    pub fn clear(&mut self) {
        self.text = None;
    }
}

pub static CLIPBOARD: IrqSpinlock<Clipboard> = IrqSpinlock::new(Clipboard::new());

/// Convenience helper to copy text into the system clipboard.
pub fn set(text: &str) {
    CLIPBOARD.lock().set_text(text);
}

/// Convenience helper to paste text from the system clipboard.
pub fn get() -> Option<String> {
    CLIPBOARD.lock().get_text()
}

/// Checks if clipboard contains text.
pub fn has() -> bool {
    CLIPBOARD.lock().has_text()
}

/// Clears clipboard content.
pub fn clear() {
    CLIPBOARD.lock().clear();
}
