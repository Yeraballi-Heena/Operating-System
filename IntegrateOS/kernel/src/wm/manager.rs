//! Central Window Manager & Z-Order Compositor
//!
//! Orchestrates window creation, destruction, focus management, hierarchical z-ordering,
//! window movement, resizing, state transitions, hit-testing, and display composition.

use alloc::vec::Vec;

use crate::graphics::canvas::Canvas;
use crate::graphics::rect::Rect;
use crate::input::cursor::VirtualCursor;
use crate::wm::render::{render_desktop, render_window};
use crate::wm::types::*;
use crate::wm::window::Window;

/// Active interaction state of the window manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionState {
    Idle,
    Moving {
        window_id: WindowId,
        mouse_start: (i32, i32),
        window_start: Rect,
    },
    Resizing {
        window_id: WindowId,
        edge: ResizeEdge,
        mouse_start: (i32, i32),
        initial_bounds: Rect,
    },
}

pub struct WindowManager {
    pub windows: Vec<Window>,
    pub active_window: Option<WindowId>,
    pub screen_bounds: Rect,
    pub interaction: InteractionState,
    next_id: u32,
}

impl WindowManager {
    /// Creates a new window manager bounded by `(width, height)`.
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            windows: Vec::new(),
            active_window: None,
            screen_bounds: Rect::new(0, 0, width, height),
            interaction: InteractionState::Idle,
            next_id: 1,
        }
    }

    /// Allocates and registers a new window, placing it on top of the z-order and granting focus.
    pub fn create_window(
        &mut self,
        title: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        flags: WindowFlags,
    ) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;

        // Deactivate all current windows
        for win in self.windows.iter_mut() {
            win.active = false;
        }

        let mut win = Window::new(id, title, x, y, width, height, flags);
        win.active = true;
        self.windows.push(win);
        self.active_window = Some(id);
        id
    }

    /// Closes and removes a window from the window manager.
    pub fn destroy_window(&mut self, id: WindowId) -> Result<(), &'static str> {
        let initial_len = self.windows.len();
        self.windows.retain(|w| w.id != id);

        if self.windows.len() == initial_len {
            return Err("Window not found (ENOENT)");
        }

        if self.active_window == Some(id) {
            self.active_window = None;
            // Activate the top-most remaining window
            if let Some(top) = self.windows.last_mut() {
                top.active = true;
                self.active_window = Some(top.id);
            }
        }

        if let InteractionState::Moving { window_id, .. } = self.interaction {
            if window_id == id {
                self.interaction = InteractionState::Idle;
            }
        }
        if let InteractionState::Resizing { window_id, .. } = self.interaction {
            if window_id == id {
                self.interaction = InteractionState::Idle;
            }
        }

        Ok(())
    }

    /// Shifts a window to the top of the z-order and marks it active.
    pub fn focus_window(&mut self, id: WindowId) -> Result<(), &'static str> {
        let idx = self
            .windows
            .iter()
            .position(|w| w.id == id)
            .ok_or("Window not found")?;

        let mut win = self.windows.remove(idx);

        for w in self.windows.iter_mut() {
            w.active = false;
        }

        win.active = true;
        self.windows.push(win);
        self.active_window = Some(id);
        Ok(())
    }

    /// Finds an immutable reference to a window by ID.
    pub fn find_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }

    /// Finds a mutable reference to a window by ID.
    pub fn find_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    /// Minimizes a window, hiding it from display.
    pub fn minimize_window(&mut self, id: WindowId) -> Result<(), &'static str> {
        let win = self.find_window_mut(id).ok_or("Window not found")?;
        win.state = WindowState::Minimized;
        win.active = false;

        if self.active_window == Some(id) {
            self.active_window = None;
            // Focus next topmost visible window
            for w in self.windows.iter_mut().rev() {
                if w.state != WindowState::Minimized && w.state != WindowState::Hidden {
                    w.active = true;
                    self.active_window = Some(w.id);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Maximizes a window to occupy the entire screen.
    pub fn maximize_window(&mut self, id: WindowId) -> Result<(), &'static str> {
        let screen_bounds = self.screen_bounds;
        let win = self.find_window_mut(id).ok_or("Window not found")?;
        if win.state != WindowState::Maximized {
            win.saved_bounds = win.bounds;
            win.bounds = screen_bounds;
            win.state = WindowState::Maximized;
            win.resize_buffer();
        }
        Ok(())
    }

    /// Restores a window from maximized or minimized state.
    pub fn restore_window(&mut self, id: WindowId) -> Result<(), &'static str> {
        let win = self.find_window_mut(id).ok_or("Window not found")?;
        if win.state == WindowState::Maximized || win.state == WindowState::Minimized {
            win.bounds = win.saved_bounds;
            win.state = WindowState::Normal;
            win.resize_buffer();
        }
        self.focus_window(id)
    }

    /// Moves a window to new `(x, y)` coordinates.
    pub fn move_window(&mut self, id: WindowId, x: i32, y: i32) -> Result<(), &'static str> {
        let win = self.find_window_mut(id).ok_or("Window not found")?;
        if !win.flags.contains(WindowFlags::MOVABLE) {
            return Err("Window is not movable");
        }
        win.bounds.x = x;
        win.bounds.y = y;
        Ok(())
    }

    /// Resizes a window to `(width, height)`.
    pub fn resize_window(&mut self, id: WindowId, width: u32, height: u32) -> Result<(), &'static str> {
        let win = self.find_window_mut(id).ok_or("Window not found")?;
        if !win.flags.contains(WindowFlags::RESIZABLE) {
            return Err("Window is not resizable");
        }
        let w = width.max(win.min_width);
        let h = height.max(win.min_height);
        win.bounds.width = w;
        win.bounds.height = h;
        win.resize_buffer();
        Ok(())
    }

    /// Performs hit-testing across all visible windows from top-to-bottom.
    pub fn hit_test(&self, px: i32, py: i32) -> Option<(WindowId, HitTestResult)> {
        for win in self.windows.iter().rev() {
            let res = win.hit_test(px, py);
            if res != HitTestResult::Nowhere {
                return Some((win.id, res));
            }
        }
        None
    }

    /// Handles mouse button press and release transitions.
    pub fn handle_mouse_button(
        &mut self,
        button: crate::input::events::MouseButton,
        action: crate::input::events::MouseButtonAction,
        pos: (i32, i32),
    ) {
        use crate::input::events::{MouseButton, MouseButtonAction};

        if button != MouseButton::Left {
            return;
        }

        match action {
            MouseButtonAction::Press => {
                if let Some((win_id, ht)) = self.hit_test(pos.0, pos.1) {
                    let _ = self.focus_window(win_id);

                    match ht {
                        HitTestResult::CloseButton => {
                            let _ = self.destroy_window(win_id);
                        }
                        HitTestResult::MaximizeButton => {
                            if let Some(win) = self.find_window(win_id) {
                                if win.state == WindowState::Maximized {
                                    let _ = self.restore_window(win_id);
                                } else {
                                    let _ = self.maximize_window(win_id);
                                }
                            }
                        }
                        HitTestResult::MinimizeButton => {
                            let _ = self.minimize_window(win_id);
                        }
                        HitTestResult::TitleBar => {
                            if let Some(win) = self.find_window(win_id) {
                                if win.flags.contains(WindowFlags::MOVABLE)
                                    && win.state != WindowState::Maximized
                                {
                                    self.interaction = InteractionState::Moving {
                                        window_id: win_id,
                                        mouse_start: pos,
                                        window_start: win.bounds,
                                    };
                                }
                            }
                        }
                        HitTestResult::ResizeBorder(edge) => {
                            if let Some(win) = self.find_window(win_id) {
                                if win.flags.contains(WindowFlags::RESIZABLE)
                                    && win.state != WindowState::Maximized
                                {
                                    self.interaction = InteractionState::Resizing {
                                        window_id: win_id,
                                        edge,
                                        mouse_start: pos,
                                        initial_bounds: win.bounds,
                                    };
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            MouseButtonAction::Release => {
                self.interaction = InteractionState::Idle;
            }
        }
    }

    /// Handles mouse motion events while moving or resizing windows.
    pub fn handle_mouse_move(&mut self, pos: (i32, i32)) {
        match self.interaction {
            InteractionState::Moving {
                window_id,
                mouse_start,
                window_start,
            } => {
                let dx = pos.0 - mouse_start.0;
                let dy = pos.1 - mouse_start.1;
                let new_x = window_start.x + dx;
                let new_y = window_start.y + dy;
                let _ = self.move_window(window_id, new_x, new_y);
            }
            InteractionState::Resizing {
                window_id,
                edge,
                mouse_start,
                initial_bounds,
            } => {
                let dx = pos.0 - mouse_start.0;
                let dy = pos.1 - mouse_start.1;
                let mut new_bounds = initial_bounds;

                match edge {
                    ResizeEdge::Right => {
                        let new_w = (initial_bounds.width as i32 + dx).max(120) as u32;
                        new_bounds.width = new_w;
                    }
                    ResizeEdge::Bottom => {
                        let new_h = (initial_bounds.height as i32 + dy).max(60) as u32;
                        new_bounds.height = new_h;
                    }
                    ResizeEdge::BottomRight => {
                        new_bounds.width = (initial_bounds.width as i32 + dx).max(120) as u32;
                        new_bounds.height = (initial_bounds.height as i32 + dy).max(60) as u32;
                    }
                    _ => {}
                }

                if let Some(win) = self.find_window_mut(window_id) {
                    win.bounds = new_bounds;
                    win.resize_buffer();
                }
            }
            InteractionState::Idle => {}
        }
    }

    /// Composites and renders the entire desktop and window stack onto `canvas`.
    pub fn render(&self, canvas: &mut Canvas) {
        // 1. Draw desktop background
        render_desktop(canvas);

        // 2. Render windows in z-order (bottom to top)
        for win in &self.windows {
            render_window(canvas, win);
        }

        // 3. Render virtual mouse cursor
        let (mx, my) = crate::input::cursor_position();
        let cursor = VirtualCursor::new(canvas.width as i32, canvas.height as i32);
        let mut cur = cursor;
        cur.set_position(mx, my);
        cur.render(|px, py, col| {
            canvas.draw_pixel(px, py, crate::graphics::color::Color32::from_u32(col));
        });
    }
}
