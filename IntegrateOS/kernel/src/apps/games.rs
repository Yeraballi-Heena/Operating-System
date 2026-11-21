//! Integrate OS — Arcade Games Suite (`GamesApp`)
//!
//! Provides playable bare-metal arcade games:
//! 1. Classic Arcade Snake (Keyboard & mouse controls, score tracking, apple spawning, audio effects)
//! 2. Space Meteor Dodge (Reflex evasion game with score timer and obstacle waves)

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::audio::speaker::PcSpeaker;
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::events::{KeyCode, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Snake,
    SpaceDodge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Ready,
    Playing,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct SnakeGame {
    pub body: Vec<(i32, i32)>,
    pub dir: Direction,
    pub next_dir: Direction,
    pub food: (i32, i32),
    pub score: u32,
    pub high_score: u32,
    pub grid_w: i32,
    pub grid_h: i32,
    pub step_timer_ms: u32,
    pub step_interval_ms: u32,
    pub state: GameState,
    pub rng_seed: u32,
}

impl SnakeGame {
    pub fn new() -> Self {
        let mut game = Self {
            body: Vec::new(),
            dir: Direction::Right,
            next_dir: Direction::Right,
            food: (12, 8),
            score: 0,
            high_score: 0,
            grid_w: 22,
            grid_h: 15,
            step_timer_ms: 0,
            step_interval_ms: 130, // Snake speed
            state: GameState::Ready,
            rng_seed: 0x1355_0848,
        };
        game.reset();
        game
    }

    fn next_rand(&mut self) -> u32 {
        self.rng_seed = self.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        self.rng_seed
    }

    pub fn reset(&mut self) {
        self.body.clear();
        self.body.push((6, 7));
        self.body.push((5, 7));
        self.body.push((4, 7));
        self.dir = Direction::Right;
        self.next_dir = Direction::Right;
        if self.score > self.high_score {
            self.high_score = self.score;
        }
        self.score = 0;
        self.state = GameState::Ready;
        self.spawn_food();
    }

    pub fn spawn_food(&mut self) {
        for _ in 0..64 {
            let rx = (self.next_rand() as i32 % self.grid_w).abs();
            let ry = (self.next_rand() as i32 % self.grid_h).abs();
            if !self.body.contains(&(rx, ry)) {
                self.food = (rx, ry);
                return;
            }
        }
        self.food = (self.grid_w / 2, self.grid_h / 2);
    }

    pub fn change_dir(&mut self, new_dir: Direction) {
        match (self.dir, new_dir) {
            (Direction::Up, Direction::Down) => {}
            (Direction::Down, Direction::Up) => {}
            (Direction::Left, Direction::Right) => {}
            (Direction::Right, Direction::Left) => {}
            _ => {
                self.next_dir = new_dir;
                if self.state == GameState::Ready {
                    self.state = GameState::Playing;
                }
            }
        }
    }

    pub fn update(&mut self, delta_ms: u32) {
        if self.state != GameState::Playing {
            return;
        }

        self.step_timer_ms += delta_ms;
        if self.step_timer_ms < self.step_interval_ms {
            return;
        }
        self.step_timer_ms = 0;

        self.dir = self.next_dir;
        let head = self.body[0];
        let new_head = match self.dir {
            Direction::Up => (head.0, head.1 - 1),
            Direction::Down => (head.0, head.1 + 1),
            Direction::Left => (head.0 - 1, head.1),
            Direction::Right => (head.0 + 1, head.1),
        };

        // Wall collision check
        if new_head.0 < 0 || new_head.0 >= self.grid_w || new_head.1 < 0 || new_head.1 >= self.grid_h {
            self.state = GameState::GameOver;
            PcSpeaker::beep(220, 200);
            return;
        }

        // Self collision check
        if self.body.contains(&new_head) {
            self.state = GameState::GameOver;
            PcSpeaker::beep(180, 250);
            return;
        }

        self.body.insert(0, new_head);

        // Check food collision
        if new_head == self.food {
            self.score += 10;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            PcSpeaker::beep(880, 40);
            self.spawn_food();
        } else {
            self.body.pop();
        }
    }
}

pub struct SpaceDodgeGame {
    pub player_x: i32,
    pub meteors: Vec<(i32, i32, i32)>, // (x, y, speed)
    pub score: u32,
    pub high_score: u32,
    pub state: GameState,
    pub spawn_timer_ms: u32,
    pub rng_seed: u32,
}

impl SpaceDodgeGame {
    pub fn new() -> Self {
        Self {
            player_x: 160,
            meteors: Vec::new(),
            score: 0,
            high_score: 0,
            state: GameState::Ready,
            spawn_timer_ms: 0,
            rng_seed: 0x7654_ABCD,
        }
    }

    fn next_rand(&mut self) -> u32 {
        self.rng_seed = self.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        self.rng_seed
    }

    pub fn reset(&mut self) {
        self.player_x = 160;
        self.meteors.clear();
        if self.score > self.high_score {
            self.high_score = self.score;
        }
        self.score = 0;
        self.state = GameState::Ready;
    }

    pub fn update(&mut self, delta_ms: u32) {
        if self.state != GameState::Playing {
            return;
        }

        self.score += delta_ms / 10;
        if self.score > self.high_score {
            self.high_score = self.score;
        }

        self.spawn_timer_ms += delta_ms;
        if self.spawn_timer_ms >= 350 {
            self.spawn_timer_ms = 0;
            let mx = ((self.next_rand() as i32 % 320) + 10).abs();
            let spd = ((self.next_rand() % 3) + 3) as i32;
            self.meteors.push((mx, 10, spd));
        }

        // Advance meteors
        for m in &mut self.meteors {
            m.1 += m.2;
        }

        // Check collisions with player at (player_x, 220)
        let px = self.player_x;
        let py = 210;
        for m in &self.meteors {
            if (m.0 - px).abs() < 18 && (m.1 - py).abs() < 14 {
                self.state = GameState::GameOver;
                PcSpeaker::beep(150, 300);
                return;
            }
        }

        // Filter offscreen
        self.meteors.retain(|m| m.1 < 240);
    }
}

pub struct GamesApp {
    pub active_mode: GameMode,
    pub snake: SnakeGame,
    pub dodge: SpaceDodgeGame,
    pub hovered_btn: Option<usize>, // 0=Snake Tab, 1=Dodge Tab, 2=Start, 3=Restart
    pub pressed_btn: Option<usize>,
    pub status_msg: String,
}

impl GamesApp {
    pub fn new() -> Self {
        Self {
            active_mode: GameMode::Snake,
            snake: SnakeGame::new(),
            dodge: SpaceDodgeGame::new(),
            hovered_btn: None,
            pressed_btn: None,
            status_msg: String::from("Ready to Play! Use Arrow Keys or WASD to navigate."),
        }
    }
}

impl Application for GamesApp {
    fn name(&self) -> &str {
        "Arcade Games"
    }

    fn on_init(&mut self, env: &mut AppEnv) -> Result<(), AppError> {
        env.set_title("Games - Arcade Hub & Retro Snake");
        env.request_redraw();
        Ok(())
    }

    fn on_update(&mut self, env: &mut AppEnv, delta_ms: u32) {
        match self.active_mode {
            GameMode::Snake => {
                if self.snake.state == GameState::Playing {
                    self.snake.update(delta_ms);
                    env.request_redraw();
                }
            }
            GameMode::SpaceDodge => {
                if self.dodge.state == GameState::Playing {
                    self.dodge.update(delta_ms);
                    env.request_redraw();
                }
            }
        }
    }

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        match *event {
            AppEvent::Input(WidgetEvent::MouseMove { x, y }) => {
                let prev_hover = self.hovered_btn;
                if (4..=28).contains(&y) {
                    if (10..130).contains(&x) {
                        self.hovered_btn = Some(0); // Snake Tab
                    } else if (136..260).contains(&x) {
                        self.hovered_btn = Some(1); // Space Dodge Tab
                    } else {
                        self.hovered_btn = None;
                    }
                } else if (290..=318).contains(&y) {
                    if (20..90).contains(&x) {
                        self.hovered_btn = Some(2); // Start/Pause
                    } else if (96..166).contains(&x) {
                        self.hovered_btn = Some(3); // Reset
                    } else {
                        self.hovered_btn = None;
                    }
                } else {
                    self.hovered_btn = None;
                }

                if prev_hover != self.hovered_btn {
                    env.request_redraw();
                }
            }
            AppEvent::Input(WidgetEvent::MouseDown { x, y, button: MouseButton::Left }) => {
                self.pressed_btn = self.hovered_btn;

                // Mode switch
                if (4..=28).contains(&y) {
                    if (10..130).contains(&x) {
                        self.active_mode = GameMode::Snake;
                        env.request_redraw();
                        return;
                    } else if (136..260).contains(&x) {
                        self.active_mode = GameMode::SpaceDodge;
                        env.request_redraw();
                        return;
                    }
                }

                // Control buttons
                if (290..=318).contains(&y) {
                    if (20..90).contains(&x) {
                        match self.active_mode {
                            GameMode::Snake => {
                                if self.snake.state == GameState::Playing {
                                    self.snake.state = GameState::Paused;
                                } else {
                                    self.snake.state = GameState::Playing;
                                }
                            }
                            GameMode::SpaceDodge => {
                                if self.dodge.state == GameState::Playing {
                                    self.dodge.state = GameState::Paused;
                                } else {
                                    self.dodge.state = GameState::Playing;
                                }
                            }
                        }
                        env.request_redraw();
                        return;
                    } else if (96..166).contains(&x) {
                        match self.active_mode {
                            GameMode::Snake => self.snake.reset(),
                            GameMode::SpaceDodge => self.dodge.reset(),
                        }
                        env.request_redraw();
                        return;
                    }
                }
            }
            AppEvent::Input(WidgetEvent::MouseUp { .. }) => {
                self.pressed_btn = None;
                env.request_redraw();
            }
            AppEvent::Input(WidgetEvent::KeyDown { code, character, .. }) => {
                match self.active_mode {
                    GameMode::Snake => {
                        match code {
                            KeyCode::Up => self.snake.change_dir(Direction::Up),
                            KeyCode::Down => self.snake.change_dir(Direction::Down),
                            KeyCode::Left => self.snake.change_dir(Direction::Left),
                            KeyCode::Right => self.snake.change_dir(Direction::Right),
                            KeyCode::Space => {
                                if self.snake.state == GameState::Playing {
                                    self.snake.state = GameState::Paused;
                                } else if self.snake.state == GameState::GameOver {
                                    self.snake.reset();
                                    self.snake.state = GameState::Playing;
                                } else {
                                    self.snake.state = GameState::Playing;
                                }
                            }
                            _ => {
                                if let Some(ch) = character {
                                    match ch {
                                        'w' | 'W' => self.snake.change_dir(Direction::Up),
                                        's' | 'S' => self.snake.change_dir(Direction::Down),
                                        'a' | 'A' => self.snake.change_dir(Direction::Left),
                                        'd' | 'D' => self.snake.change_dir(Direction::Right),
                                        'r' | 'R' => self.snake.reset(),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        env.request_redraw();
                    }
                    GameMode::SpaceDodge => {
                        match code {
                            KeyCode::Left => self.dodge.player_x = (self.dodge.player_x - 16).max(20),
                            KeyCode::Right => self.dodge.player_x = (self.dodge.player_x + 16).min(330),
                            KeyCode::Space => {
                                if self.dodge.state == GameState::Playing {
                                    self.dodge.state = GameState::Paused;
                                } else if self.dodge.state == GameState::GameOver {
                                    self.dodge.reset();
                                    self.dodge.state = GameState::Playing;
                                } else {
                                    self.dodge.state = GameState::Playing;
                                }
                            }
                            _ => {
                                if let Some(ch) = character {
                                    if ch == 'a' || ch == 'A' {
                                        self.dodge.player_x = (self.dodge.player_x - 16).max(20);
                                    } else if ch == 'd' || ch == 'D' {
                                        self.dodge.player_x = (self.dodge.player_x + 16).min(330);
                                    } else if ch == 'r' || ch == 'R' {
                                        self.dodge.reset();
                                    }
                                }
                            }
                        }
                        env.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}

    fn render(&self, canvas: &mut Canvas) {
        let w = canvas.width;
        let h = canvas.height;

        // Dark arcade cabinet background
        canvas.draw_rect_fill(Rect::new(0, 0, w, h), Color32::rgb(0x13, 0x11, 0x1C));

        // Top Navigation Tabs
        canvas.draw_rect_fill(Rect::new(0, 0, w, 32), Color32::rgb(0x1E, 0x1A, 0x2E));
        canvas.draw_hline(0, 31, w, Color32::rgb(0x3B, 0x33, 0x58));

        let tabs = [
            (10, 116, "1:Arcade Snake", GameMode::Snake, 0),
            (134, 122, "2:Space Dodge", GameMode::SpaceDodge, 1),
        ];

        for &(tx, tw, label, mode, btn_id) in &tabs {
            let is_sel = self.active_mode == mode;
            let is_hover = self.hovered_btn == Some(btn_id);
            let bg = if is_sel {
                Color32::rgb(0x7C, 0x3A, 0xED)
            } else if is_hover {
                Color32::rgb(0x4C, 0x1D, 0x95)
            } else {
                Color32::rgb(0x28, 0x22, 0x3E)
            };
            canvas.draw_rect_fill(Rect::new(tx, 4, tw, 24), bg);
            canvas.draw_rect_outline(Rect::new(tx, 4, tw, 24), if is_sel { Color32::rgb(0xC4, 0xB5, 0xFD) } else { Color32::rgb(0x6D, 0x28, 0xD9) });
            canvas.draw_text(tx + 8, 8, label, Color32::WHITE, None);
        }

        // =====================================================================
        // ARCADE SCREEN FIELD (352x240 pixels at x=16, y=42)
        // =====================================================================
        let screen_rect = Rect::new(16, 42, 352, 240);
        canvas.draw_rect_fill(screen_rect, Color32::rgb(0x09, 0x07, 0x10));
        canvas.draw_rect_outline(screen_rect, Color32::rgb(0x8B, 0x5C, 0xF6));

        match self.active_mode {
            GameMode::Snake => {
                let cell_size = 16i32;

                // Subtle grid dots
                for gy in 0..self.snake.grid_h {
                    for gx in 0..self.snake.grid_w {
                        let dot_x = 16 + gx * cell_size + 8;
                        let dot_y = 42 + gy * cell_size + 8;
                        canvas.draw_pixel(dot_x, dot_y, Color32::rgb(0x1E, 0x1A, 0x2E));
                    }
                }

                // Draw Apple Food (Red orb with green leaf)
                let fx = 16 + self.snake.food.0 * cell_size;
                let fy = 42 + self.snake.food.1 * cell_size;
                canvas.draw_rect_fill(Rect::new(fx + 2, fy + 3, 12, 11), Color32::rgb(0xEF, 0x44, 0x44));
                canvas.draw_rect_fill(Rect::new(fx + 6, fy + 1, 4, 3), Color32::rgb(0x10, 0xB9, 0x81)); // Leaf

                // Draw Snake Body
                for (i, segment) in self.snake.body.iter().enumerate() {
                    let sx = 16 + segment.0 * cell_size;
                    let sy = 42 + segment.1 * cell_size;

                    if i == 0 {
                        // Head: Bright Lime Green with Eye dots
                        canvas.draw_rect_fill(Rect::new(sx + 1, sy + 1, 14, 14), Color32::rgb(0x4A, 0xDE, 0x80));
                        canvas.draw_rect_outline(Rect::new(sx + 1, sy + 1, 14, 14), Color32::rgb(0x16, 0x65, 0x34));
                        // Eyes
                        canvas.draw_pixel(sx + 4, sy + 4, Color32::BLACK);
                        canvas.draw_pixel(sx + 11, sy + 4, Color32::BLACK);
                    } else {
                        // Body segment: Emerald green with inner bevel
                        let col = if i % 2 == 0 { Color32::rgb(0x10, 0xB9, 0x81) } else { Color32::rgb(0x05, 0x96, 0x69) };
                        canvas.draw_rect_fill(Rect::new(sx + 2, sy + 2, 12, 12), col);
                        canvas.draw_rect_outline(Rect::new(sx + 2, sy + 2, 12, 12), Color32::rgb(0x06, 0x4E, 0x3B));
                    }
                }

                // Overlays for Ready, Paused, GameOver
                match self.snake.state {
                    GameState::Ready => {
                        canvas.draw_rect_fill(Rect::new(80, 120, 224, 60), Color32::rgba(15, 23, 42, 220));
                        canvas.draw_rect_outline(Rect::new(80, 120, 224, 60), Color32::rgb(0x38, 0xBD, 0xF8));
                        canvas.draw_text(110, 132, "PRESS ARROWS OR WASD", Color32::WHITE, None);
                        canvas.draw_text(130, 152, "TO START SNAKE", Color32::rgb(0x34, 0xD3, 0x99), None);
                    }
                    GameState::Paused => {
                        canvas.draw_rect_fill(Rect::new(110, 130, 164, 46), Color32::rgba(15, 23, 42, 230));
                        canvas.draw_rect_outline(Rect::new(110, 130, 164, 46), Color32::rgb(0xFB, 0xBF, 0x24));
                        canvas.draw_text(150, 144, "GAME PAUSED", Color32::WHITE, None);
                    }
                    GameState::GameOver => {
                        canvas.draw_rect_fill(Rect::new(80, 110, 224, 76), Color32::rgba(20, 10, 30, 235));
                        canvas.draw_rect_outline(Rect::new(80, 110, 224, 76), Color32::rgb(0xEF, 0x44, 0x44));
                        canvas.draw_text(142, 122, "GAME OVER", Color32::rgb(0xEF, 0x44, 0x44), None);
                        canvas.draw_text(108, 142, &format!("Final Score: {} Pts", self.snake.score), Color32::WHITE, None);
                        canvas.draw_text(96, 160, "Press Space / R to Play Again", Color32::rgb(0xC4, 0xB5, 0xFD), None);
                    }
                    GameState::Playing => {}
                }
            }
            GameMode::SpaceDodge => {
                // Starfield background
                for i in 0..24 {
                    let star_x = 24 + ((i * 47) % 330);
                    let star_y = 48 + ((i * 29) % 220);
                    canvas.draw_pixel(star_x, star_y, Color32::WHITE);
                }

                // Draw Meteors
                for m in &self.dodge.meteors {
                    canvas.draw_rect_fill(Rect::new(m.0, m.1, 14, 14), Color32::rgb(0xF9, 0x73, 0x16));
                    canvas.draw_rect_outline(Rect::new(m.0, m.1, 14, 14), Color32::rgb(0xEA, 0x58, 0x0C));
                    canvas.draw_pixel(m.0 + 4, m.1 + 4, Color32::rgb(0xFE, 0xF0, 0x8A));
                }

                // Draw Player Spacecraft at (player_x, 210)
                let px = self.dodge.player_x;
                let py = 210;
                canvas.draw_rect_fill(Rect::new(px - 10, py, 20, 10), Color32::rgb(0x38, 0xBD, 0xF8));
                canvas.draw_rect_fill(Rect::new(px - 4, py - 6, 8, 8), Color32::rgb(0x60, 0xA5, 0xFA));
                canvas.draw_rect_fill(Rect::new(px - 6, py + 10, 12, 4), Color32::rgb(0xEF, 0x44, 0x44)); // Thruster flame

                if self.dodge.state == GameState::GameOver {
                    canvas.draw_rect_fill(Rect::new(80, 110, 224, 76), Color32::rgba(20, 10, 30, 235));
                    canvas.draw_rect_outline(Rect::new(80, 110, 224, 76), Color32::rgb(0xEF, 0x44, 0x44));
                    canvas.draw_text(142, 122, "SHIP DESTROYED", Color32::rgb(0xEF, 0x44, 0x44), None);
                    canvas.draw_text(116, 142, &format!("Survival: {} Pts", self.dodge.score), Color32::WHITE, None);
                    canvas.draw_text(96, 160, "Press Space / R to Launch", Color32::rgb(0x38, 0xBD, 0xF8), None);
                }
            }
        }

        // =====================================================================
        // RIGHT SIDE STATS PANEL (x=380..w)
        // =====================================================================
        let stats_rect = Rect::new(380, 42, w.saturating_sub(394), 240);
        canvas.draw_rect_fill(stats_rect, Color32::rgb(0x1A, 0x16, 0x28));
        canvas.draw_rect_outline(stats_rect, Color32::rgb(0x3B, 0x33, 0x58));

        canvas.draw_text(390, 50, "SCOREBOARD", Color32::rgb(0xC4, 0xB5, 0xFD), None);
        let cur_score = match self.active_mode {
            GameMode::Snake => self.snake.score,
            GameMode::SpaceDodge => self.dodge.score,
        };
        let high_score = match self.active_mode {
            GameMode::Snake => self.snake.high_score,
            GameMode::SpaceDodge => self.dodge.high_score,
        };
        canvas.draw_text(390, 72, &format!("Score: {}", cur_score), Color32::rgb(0x34, 0xD3, 0x99), None);
        canvas.draw_text(390, 92, &format!("High : {}", high_score), Color32::rgb(0xFB, 0xBF, 0x24), None);

        canvas.draw_text(390, 122, "CONTROLS:", Color32::rgb(0x94, 0xA3, 0xB8), None);
        canvas.draw_text(390, 140, "Arrows / WASD : Move", Color32::WHITE, None);
        canvas.draw_text(390, 158, "Space : Pause / Resume", Color32::WHITE, None);
        canvas.draw_text(390, 176, "R Key : Restart Game", Color32::WHITE, None);

        // Hardware Sound Indicator
        canvas.draw_rect_fill(Rect::new(390, 210, 130, 26), Color32::rgb(0x0F, 0x17, 0x2A));
        canvas.draw_rect_outline(Rect::new(390, 210, 130, 26), Color32::rgb(0x10, 0xB9, 0x81));
        canvas.draw_text(396, 216, "🔊 PC Speaker: ON", Color32::rgb(0x34, 0xD3, 0x99), None);

        // =====================================================================
        // BOTTOM BUTTONS & CONTROLS with Animations
        // =====================================================================
        let btns = [
            (20, 70, "Start / Pause", 2),
            (98, 70, "Restart (R)", 3),
        ];

        for &(bx, bw, label, btn_id) in &btns {
            let is_hover = self.hovered_btn == Some(btn_id);
            let is_pressed = self.pressed_btn == Some(btn_id);
            let off = if is_pressed { 1 } else { 0 };

            let bg = if is_pressed {
                Color32::rgb(0x58, 0x1C, 0x87)
            } else if is_hover {
                Color32::rgb(0x7C, 0x3A, 0xED)
            } else {
                Color32::rgb(0x3B, 0x1A, 0x6E)
            };
            canvas.draw_rect_fill(Rect::new(bx + off, 290 + off, bw, 26), bg);
            canvas.draw_rect_outline(Rect::new(bx + off, 290 + off, bw, 26), if is_hover { Color32::rgb(0xC4, 0xB5, 0xFD) } else { Color32::rgb(0x6D, 0x28, 0xD9) });
            canvas.draw_text(bx + 6 + off, 295 + off, label, Color32::WHITE, None);
        }

        // Footer banner
        canvas.draw_rect_fill(Rect::new(0, (h.saturating_sub(24)) as i32, w, 24), Color32::rgb(0x0C, 0x0A, 0x14));
        canvas.draw_text(16, (h.saturating_sub(18)) as i32, &self.status_msg, Color32::rgb(0xC4, 0xB5, 0xFD), None);
    }
}
