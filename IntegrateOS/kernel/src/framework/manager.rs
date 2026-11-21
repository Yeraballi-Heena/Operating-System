//! Integrate OS — Application Lifecycle Manager & Runtime Dispatcher
//!
//! Orchestrates application spawning, window binding, event routing,
//! timer ticking, and offscreen UI container rasterization.

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::graphics::canvas::Canvas;
use crate::gui::types::EventResponse;
use crate::gui::widget::Widget;
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::{AppError, AppState, InstanceId};
use crate::wm::types::*;
use crate::wm::WindowManager;

/// An active application instance running within the OS runtime.
pub struct AppInstance {
    pub id: InstanceId,
    pub state: AppState,
    pub app: Box<dyn Application>,
    pub env: AppEnv,
}

/// Central application runtime and lifecycle coordinator.
pub struct AppManager {
    pub instances: Vec<AppInstance>,
}

impl AppManager {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    /// Spawns a new application instance, creating its host window and initializing its UI.
    pub fn spawn<A: Application + 'static>(
        &mut self,
        mut app: A,
        wm: &mut WindowManager,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<InstanceId, AppError> {
        let name = app.name();
        let win_id = wm.create_window(name, x, y, width, height, WindowFlags::DEFAULT);

        let win = wm.find_window_mut(win_id).ok_or(AppError::WindowCreationFailed)?;
        let (cw, ch) = win.client_size();

        let inst_id = InstanceId::next();
        let mut env = AppEnv::new(inst_id, win_id, 0, cw, ch);
        env.set_title(name);

        if let Err(e) = app.on_init(&mut env) {
            let _ = wm.destroy_window(win_id);
            return Err(e);
        }

        // Update window title if changed by app
        if !env.title.is_empty() {
            if let Some(w) = wm.find_window_mut(win_id) {
                w.title = env.title.clone();
            }
        }

        let mut instance = AppInstance {
            id: inst_id,
            state: AppState::Running,
            app: Box::new(app),
            env,
        };

        // Render initial UI container into window client buffer
        Self::render_instance_ui(&mut instance, wm);

        self.instances.push(instance);
        let _ = wm.focus_window(win_id);

        Ok(inst_id)
    }

    /// Terminates an application instance and closes its associated window.
    pub fn terminate(&mut self, id: InstanceId, wm: &mut WindowManager) -> Result<(), AppError> {
        let idx = self
            .instances
            .iter()
            .position(|inst| inst.id == id)
            .ok_or(AppError::InstanceNotFound)?;

        let mut instance = self.instances.remove(idx);
        instance.state = AppState::Terminated;
        instance.app.on_destroy(&mut instance.env);

        let _ = wm.destroy_window(instance.env.window_id);
        Ok(())
    }

    /// Suspends a running application instance.
    pub fn suspend(&mut self, id: InstanceId) -> Result<(), AppError> {
        let inst = self.find_mut(id).ok_or(AppError::InstanceNotFound)?;
        if inst.state == AppState::Running {
            inst.state = AppState::Suspended;
            Ok(())
        } else {
            Err(AppError::AlreadyTerminated)
        }
    }

    /// Resumes a suspended application instance.
    pub fn resume(&mut self, id: InstanceId) -> Result<(), AppError> {
        let inst = self.find_mut(id).ok_or(AppError::InstanceNotFound)?;
        if inst.state == AppState::Suspended {
            inst.state = AppState::Running;
            Ok(())
        } else {
            Err(AppError::AlreadyTerminated)
        }
    }

    /// Dispatches an event to an active application instance.
    pub fn dispatch_event(
        &mut self,
        id: InstanceId,
        event: &AppEvent,
        wm: &mut WindowManager,
    ) -> Result<(), AppError> {
        let inst = self.find_mut(id).ok_or(AppError::InstanceNotFound)?;
        if inst.state != AppState::Running {
            return Ok(());
        }

        // 1. If event is low-level Input, dispatch to UI container first
        if let AppEvent::Input(ref we) = *event {
            let res = inst.env.dispatch_widget_event(we);
            if let EventResponse::Action(action_id) = res {
                inst.app.on_event(&mut inst.env, &AppEvent::Action(action_id));
            }
        }

        // 2. Dispatch to application's on_event
        inst.app.on_event(&mut inst.env, event);

        // 3. Check if redraw is needed
        if inst.env.redraw_requested {
            Self::render_instance_ui(inst, wm);
        }

        // 4. Check if quit was requested
        if inst.env.quit_requested {
            self.terminate(id, wm)?;
        }

        Ok(())
    }

    /// Updates all running applications with delta time.
    /// Returns true if any application requested a redraw.
    pub fn update_all(&mut self, delta_ms: u32, wm: &mut WindowManager) -> bool {
        let mut to_terminate = Vec::new();
        let mut any_redrawn = false;

        for inst in &mut self.instances {
            if inst.state != AppState::Running {
                continue;
            }

            inst.app.on_update(&mut inst.env, delta_ms);

            if inst.env.redraw_requested {
                Self::render_instance_ui(inst, wm);
                any_redrawn = true;
            }

            if inst.env.quit_requested {
                to_terminate.push(inst.id);
            }
        }

        for id in to_terminate {
            let _ = self.terminate(id, wm);
        }

        any_redrawn
    }

    /// Renders an instance's root UI container onto its window client surface.
    pub fn render_instance_ui(instance: &mut AppInstance, wm: &mut WindowManager) {
        if let Some(win) = wm.find_window_mut(instance.env.window_id) {
            let (cw, ch) = win.client_size();
            if cw > 0 && ch > 0 {
                let mut canvas = Canvas::new(cw, ch, cw * 4, 32, 0);
                instance.env.ui_root.render(&mut canvas);
                instance.app.render(&mut canvas);

                if win.content_buffer.len() == canvas.back_buffer.len() {
                    win.content_buffer.copy_from_slice(&canvas.back_buffer);
                }
                drop(canvas);
            }
            instance.env.redraw_requested = false;
        }
    }

    pub fn find(&self, id: InstanceId) -> Option<&AppInstance> {
        self.instances.iter().find(|inst| inst.id == id)
    }

    pub fn find_mut(&mut self, id: InstanceId) -> Option<&mut AppInstance> {
        self.instances.iter_mut().find(|inst| inst.id == id)
    }

    pub fn find_by_window(&self, win_id: WindowId) -> Option<InstanceId> {
        self.instances
            .iter()
            .find(|inst| inst.env.window_id == win_id)
            .map(|inst| inst.id)
    }

    pub fn count(&self) -> usize {
        self.instances.len()
    }
}
