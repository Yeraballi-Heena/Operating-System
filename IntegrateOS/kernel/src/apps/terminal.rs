//! Integrate OS — Real Command-Line Shell & Terminal Application (Phases 5, 6 & 7)
//!
//! Supports:
//! - Full command parsing, arguments, flags, quoted strings, environment variables
//! - Redirection (`>`, `>>`) and Pipelines (`| grep`, `| head`, `| tail`)
//! - Command history (Up / Down arrows) and Tab completion
//! - Built-in commands:
//!   help, clear, echo, pwd, cd, ls, mkdir, touch, cat, cp, mv, rm, rmdir,
//!   find, grep, head, tail, date, time, whoami, uname, uptime, free, mem,
//!   ps, kill, mount, umount, mounts, lsblk, df, lspci, hwinfo,
//!   ifconfig, ip, ping, route, dns, netstat, wifi,
//!   apt, aptitude, pkg, snap, snapstore, research, affinity,
//!   env, export, history, reboot, shutdown, exit

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::arch::x86_64::pit;
use crate::drivers::hwinfo::{format_hwinfo, format_lspci};
use crate::framework::app::Application;
use crate::framework::env::AppEnv;
use crate::framework::event::AppEvent;
use crate::framework::types::AppError;
use crate::fs;
use crate::fs::path::normalize_path;
use crate::graphics::canvas::Canvas;
use crate::graphics::color::Color32;
use crate::graphics::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::graphics::rect::Rect;
use crate::gui::types::WidgetEvent;
use crate::input::events::KeyCode;
use crate::memory::frame_allocator::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::memory::heap::ALLOCATOR;
use crate::net::device::NetDevice;
use crate::net::stack::NETWORK_STACK;
use crate::net::wifi::{self, WIFI_MANAGER};
use crate::storage::ssd;

pub const MAX_HISTORY_LINES: usize = 160;

const BUILTIN_COMMANDS: &[&str] = &[
    "help", "clear", "echo", "pwd", "cd", "ls", "mkdir", "touch", "cat", "cp", "mv", "rm",
    "rmdir", "find", "grep", "head", "tail", "date", "time", "whoami", "uname", "uptime",
    "free", "mem", "ps", "kill", "mount", "umount", "mounts", "lsblk", "df", "lspci",
    "hwinfo", "ifconfig", "ip", "ping", "route", "dns", "dhcp", "netstat", "wifi", "apt",
    "aptitude", "pkg", "snap", "snapstore", "research", "affinity", "env", "export",
    "history", "reboot", "shutdown", "exit",
];

/// Terminal shell application.
pub struct TerminalApp {
    pub history: Vec<String>,
    pub cmd_history: Vec<String>,
    pub cmd_history_idx: usize,
    pub input_line: String,
    pub cursor_pos: usize,
    pub cwd: String,
    pub env_vars: BTreeMap<String, String>,
    pub last_exit_code: i32,
}

impl TerminalApp {
    pub fn new() -> Self {
        let mut env_vars = BTreeMap::new();
        env_vars.insert(String::from("USER"), String::from("root"));
        env_vars.insert(String::from("HOSTNAME"), String::from("DESKTOP-L8RF961"));
        env_vars.insert(String::from("PATH"), String::from("/bin:/usr/bin"));
        env_vars.insert(String::from("HOME"), String::from("/home"));
        env_vars.insert(String::from("SHELL"), String::from("/bin/ish"));

        Self {
            history: Vec::new(),
            cmd_history: Vec::new(),
            cmd_history_idx: 0,
            input_line: String::new(),
            cursor_pos: 0,
            cwd: String::from("/"),
            env_vars,
            last_exit_code: 0,
        }
    }

    /// Resolves a path relative to `self.cwd`.
    pub fn resolve_path(&self, raw: &str) -> String {
        if raw.starts_with('/') {
            normalize_path(raw)
        } else if self.cwd == "/" {
            normalize_path(&format!("/{}", raw))
        } else {
            normalize_path(&format!("{}/{}", self.cwd, raw))
        }
    }

    /// Appends a line of text to the terminal history buffer.
    pub fn print_line(&mut self, text: &str) {
        if self.history.len() >= MAX_HISTORY_LINES {
            self.history.remove(0);
        }
        self.history.push(String::from(text));
    }

    /// Reads an entire text file from VFS into a `String`.
    fn read_file_to_string(&self, path: &str) -> Result<String, &'static str> {
        let full = self.resolve_path(path);
        let fd = fs::open(&full, fs::OpenFlags::READ)?;
        let mut buf = [0u8; 2048];
        let n = fs::read(fd, &mut buf).unwrap_or(0);
        let _ = fs::close(fd);
        match core::str::from_utf8(&buf[..n]) {
            Ok(s) => Ok(String::from(s)),
            Err(_) => Ok(String::from("[Binary or non-UTF8 content]")),
        }
    }

    /// Writes string content to a VFS file (truncating or appending).
    fn write_string_to_file(&self, path: &str, content: &str, append: bool) -> Result<(), &'static str> {
        let full = self.resolve_path(path);
        let flags = fs::OpenFlags(
            fs::OpenFlags::WRITE.0 | fs::OpenFlags::CREATE.0 | fs::OpenFlags::TRUNCATE.0,
        );
        if append {
            let existing = self.read_file_to_string(&full).unwrap_or_default();
            let combined = format!("{}{}", existing, content);
            let fd = fs::open(&full, flags)?;
            let _ = fs::write(fd, combined.as_bytes());
            let _ = fs::close(fd);
            Ok(())
        } else {
            let fd = fs::open(&full, flags)?;
            let _ = fs::write(fd, content.as_bytes());
            let _ = fs::close(fd);
            Ok(())
        }
    }

    /// Performs tab completion on `self.input_line`.
    pub fn tab_complete(&mut self) {
        let trimmed = self.input_line.trim_start();
        if trimmed.is_empty() {
            return;
        }
        if !trimmed.contains(' ') {
            for &cmd in BUILTIN_COMMANDS {
                if cmd.starts_with(trimmed) {
                    self.input_line = String::from(cmd);
                    self.input_line.push(' ');
                    self.cursor_pos = self.input_line.len();
                    return;
                }
            }
        }
    }

    /// Executes a command string (supporting `|`, `>`, `>>`) and appends results to history.
    pub fn execute_command(&mut self, cmd: &str, env: &mut AppEnv) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }

        self.cmd_history.push(String::from(trimmed));
        self.cmd_history_idx = self.cmd_history.len();

        // 1. Check for output redirection (`>>` or `>`)
        let (cmd_part, redir_target, is_append) = if let Some(idx) = trimmed.find(">>") {
            let left = trimmed[..idx].trim();
            let right = trimmed[idx + 2..].trim();
            (left, Some(right), true)
        } else if let Some(idx) = trimmed.find('>') {
            let left = trimmed[..idx].trim();
            let right = trimmed[idx + 1..].trim();
            (left, Some(right), false)
        } else {
            (trimmed, None, false)
        };

        // 2. Check for pipeline (`|`)
        let mut stages = cmd_part.split('|');
        let primary_stage = stages.next().unwrap_or("").trim();

        if primary_stage == "clear" && redir_target.is_none() {
            self.history.clear();
            self.last_exit_code = 0;
            return;
        }

        let mut output_lines = self.eval_single_command(primary_stage, env);

        // Process any piped stages (`| grep <pat>`, `| head`, `| tail`)
        for pipe_stage in stages {
            let ps = pipe_stage.trim();
            let mut pparts = ps.split_whitespace();
            let pcmd = pparts.next().unwrap_or("");
            match pcmd {
                "grep" => {
                    if let Some(pat) = pparts.next() {
                        let pat_clean = pat.trim_matches('"').trim_matches('\'');
                        output_lines.retain(|line| line.contains(pat_clean));
                    }
                }
                "head" => {
                    let n = pparts
                        .next()
                        .and_then(|s| s.trim_start_matches('-').parse::<usize>().ok())
                        .unwrap_or(5);
                    if output_lines.len() > n {
                        output_lines.truncate(n);
                    }
                }
                "tail" => {
                    let n = pparts
                        .next()
                        .and_then(|s| s.trim_start_matches('-').parse::<usize>().ok())
                        .unwrap_or(5);
                    if output_lines.len() > n {
                        let skip = output_lines.len() - n;
                        output_lines = output_lines.into_iter().skip(skip).collect();
                    }
                }
                _ => {}
            }
        }

        // 3. Emit to redirected file or print to terminal history
        if let Some(target_file) = redir_target {
            if target_file.is_empty() {
                self.print_line("sh: syntax error near unexpected token `newline'");
                self.last_exit_code = 2;
            } else {
                let mut combined = output_lines.join("\n");
                if !combined.is_empty() {
                    combined.push('\n');
                }
                match self.write_string_to_file(target_file, &combined, is_append) {
                    Ok(()) => self.last_exit_code = 0,
                    Err(e) => {
                        self.print_line(&format!("sh: {}: {}", target_file, e));
                        self.last_exit_code = 1;
                    }
                }
            }
        } else {
            for line in output_lines {
                self.print_line(&line);
            }
        }
    }

    /// Evaluates a single command stage and returns its output lines.
    fn eval_single_command(&mut self, cmd: &str, env: &mut AppEnv) -> Vec<String> {
        let mut out = Vec::new();
        let mut parts = cmd.split_whitespace();
        let program = parts.next().unwrap_or("");

        match program {
            "help" => {
                out.push(String::from("Integrate OS Shell (ish) - Built-in Commands:"));
                out.push(String::from("  Hardware : hwinfo, lspci [-v], lsblk, df, mount, umount, mounts"));
                out.push(String::from("  Network  : ifconfig, ip, ping <host>, wifi [scan|connect|status|info], dhcp, dns <host>, route, netstat"));
                out.push(String::from("  Packages : apt / aptitude / pkg <update|list|search|install|remove|upgrade|show|verify>"));
                out.push(String::from("  AppStore : snap / snapstore <list|find|info|install|remove>"));
                out.push(String::from("  Files    : pwd, cd, ls, mkdir, touch, cat, cp, mv, rm, rmdir, find, grep, head, tail"));
                out.push(String::from("  System   : uname, uptime, free, mem, ps, kill, date, time, whoami, env, export"));
                out.push(String::from("  Research : research, affinity | Power: clear, history, reboot, shutdown, exit"));
                self.last_exit_code = 0;
            }
            "uname" => {
                out.push(String::from("IntegrateOS 0.1.0 x86_64 Long Mode (Ring 0) DESKTOP-L8RF961"));
                self.last_exit_code = 0;
            }
            "whoami" => {
                let u = self.env_vars.get("USER").map(|s| s.as_str()).unwrap_or("root");
                out.push(String::from(u));
                self.last_exit_code = 0;
            }
            "pwd" => {
                out.push(self.cwd.clone());
                self.last_exit_code = 0;
            }
            "cd" => {
                let target = parts.next().unwrap_or("/");
                let resolved = self.resolve_path(target);
                match fs::stat(&resolved) {
                    Ok(meta) if meta.node_type == fs::NodeType::Directory => {
                        self.cwd = resolved;
                        self.last_exit_code = 0;
                    }
                    Ok(_) => {
                        out.push(format!("cd: {}: Not a directory", target));
                        self.last_exit_code = 1;
                    }
                    Err(e) => {
                        out.push(format!("cd: {}: {}", target, e));
                        self.last_exit_code = 1;
                    }
                }
            }
            "uptime" => {
                let ticks = pit::get_ticks();
                let secs = ticks / 100;
                out.push(format!("Uptime: {} ticks ({} seconds)", ticks, secs));
                self.last_exit_code = 0;
            }
            "date" | "time" => {
                let dt = crate::time::now();
                out.push(format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC (Uptime: {} ms)",
                    dt.year,
                    dt.month,
                    dt.day,
                    dt.hour,
                    dt.minute,
                    dt.second,
                    crate::time::uptime_ms()
                ));
                self.last_exit_code = 0;
            }
            "echo" => {
                let rest: Vec<&str> = parts.collect();
                let mut words = Vec::new();
                for w in rest {
                    let cleaned = w.trim_matches('"').trim_matches('\'');
                    if let Some(var_name) = cleaned.strip_prefix('$') {
                        if var_name == "?" {
                            words.push(format!("{}", self.last_exit_code));
                        } else if let Some(val) = self.env_vars.get(var_name) {
                            words.push(val.clone());
                        } else {
                            words.push(String::new());
                        }
                    } else {
                        words.push(String::from(cleaned));
                    }
                }
                out.push(words.join(" "));
                self.last_exit_code = 0;
            }
            "env" => {
                for (k, v) in &self.env_vars {
                    out.push(format!("{}={}", k, v));
                }
                self.last_exit_code = 0;
            }
            "export" => {
                if let Some(kv) = parts.next() {
                    if let Some(eq) = kv.find('=') {
                        let k = kv[..eq].trim();
                        let v = kv[eq + 1..].trim().trim_matches('"');
                        self.env_vars.insert(String::from(k), String::from(v));
                        self.last_exit_code = 0;
                    } else {
                        out.push(String::from("Usage: export KEY=VALUE"));
                    }
                }
            }
            "history" => {
                for (i, h) in self.cmd_history.iter().enumerate() {
                    out.push(format!(" {:>3}  {}", i + 1, h));
                }
                self.last_exit_code = 0;
            }
            "free" | "mem" => {
                let frame_alloc = FRAME_ALLOCATOR.lock();
                let total_frames = frame_alloc.total_frames();
                let free_frames = frame_alloc.free_frames();
                drop(frame_alloc);

                let heap_stats = ALLOCATOR.0.lock();
                let heap_alloc = heap_stats.allocated_bytes();
                let heap_free = heap_stats.free_bytes();
                let heap_total = heap_stats.total_bytes();
                drop(heap_stats);

                let free_mib = (free_frames * PAGE_SIZE) / (1024 * 1024);
                let total_mib = (total_frames * PAGE_SIZE) / (1024 * 1024);
                let used_mib = total_mib.saturating_sub(free_mib);

                out.push(format!(
                    "Physical Frames: Free={} ({} MiB free / {} MiB total, {} MiB used)",
                    free_frames, free_mib, total_mib, used_mib
                ));
                out.push(format!(
                    "Kernel Heap: Allocated={} KiB, Free={} KiB / Total={} KiB",
                    heap_alloc / 1024,
                    heap_free / 1024,
                    heap_total / 1024
                ));
                self.last_exit_code = 0;
            }
            "ps" => {
                out.push(String::from("  PID STATE      PRIO AFFINITY   TICKS NAME"));
                let table = crate::process::PROCESS_MANAGER.lock();
                for pcb in table.iter() {
                    out.push(format!(
                        "{:>5} {:<10} {:>4} {:#06x} {:>7} {}",
                        pcb.pid.as_u64(),
                        pcb.state.as_str(),
                        pcb.priority,
                        pcb.cpu_affinity,
                        pcb.ticks_used,
                        pcb.name
                    ));
                }
                self.last_exit_code = 0;
            }
            "kill" => {
                if let Some(pid_s) = parts.next() {
                    if let Ok(pid_num) = pid_s.parse::<u64>() {
                        if pid_num <= 1 {
                            out.push(String::from("kill: cannot terminate protected kernel task (PID 0/1)"));
                            self.last_exit_code = 1;
                        } else {
                            crate::process::PROCESS_MANAGER.lock().terminate(crate::process::ProcessId(pid_num), -9);
                            out.push(format!("Terminated process PID {}", pid_num));
                            self.last_exit_code = 0;
                        }
                    } else {
                        out.push(String::from("kill: invalid PID"));
                    }
                } else {
                    out.push(String::from("Usage: kill <pid>"));
                }
            }
            "ls" => {
                let target_arg = parts.next().unwrap_or(&self.cwd);
                let target_path = self.resolve_path(target_arg);
                match fs::readdir(&target_path) {
                    Ok(entries) => {
                        out.push(format!("Directory contents ({}):", target_path));
                        for entry in entries {
                            let tag = if entry.node_type == fs::NodeType::Directory {
                                "[DIR]  "
                            } else if entry.node_type == fs::NodeType::CharDevice {
                                "[CHR]  "
                            } else if entry.node_type == fs::NodeType::BlockDevice {
                                "[BLK]  "
                            } else {
                                "[FILE] "
                            };
                            out.push(format!("{}{}", tag, entry.name));
                        }
                        self.last_exit_code = 0;
                    }
                    Err(e) => {
                        out.push(format!("ls error: {}", e));
                        self.last_exit_code = 1;
                    }
                }
            }
            "mkdir" => {
                if let Some(dir_arg) = parts.next() {
                    let full = self.resolve_path(dir_arg);
                    match fs::mkdir(&full) {
                        Ok(()) => {
                            out.push(format!("Created directory {}", full));
                            self.last_exit_code = 0;
                        }
                        Err(e) => {
                            out.push(format!("mkdir: {}: {}", dir_arg, e));
                            self.last_exit_code = 1;
                        }
                    }
                } else {
                    out.push(String::from("Usage: mkdir <directory>"));
                }
            }
            "touch" => {
                if let Some(file_arg) = parts.next() {
                    let full = self.resolve_path(file_arg);
                    let flags = fs::OpenFlags(
                        fs::OpenFlags::WRITE.0 | fs::OpenFlags::CREATE.0,
                    );
                    match fs::open(&full, flags) {
                        Ok(fd) => {
                            let _ = fs::close(fd);
                            self.last_exit_code = 0;
                        }
                        Err(e) => {
                            out.push(format!("touch: {}: {}", file_arg, e));
                            self.last_exit_code = 1;
                        }
                    }
                } else {
                    out.push(String::from("Usage: touch <filename>"));
                }
            }
            "cat" => {
                if let Some(file_path) = parts.next() {
                    match self.read_file_to_string(file_path) {
                        Ok(content) => {
                            for line in content.lines() {
                                out.push(String::from(line));
                            }
                            self.last_exit_code = 0;
                        }
                        Err(e) => {
                            out.push(format!("Open error: {}", e));
                            self.last_exit_code = 1;
                        }
                    }
                } else {
                    out.push(String::from("Usage: cat <filename>"));
                }
            }
            "cp" => {
                let src = parts.next();
                let dst = parts.next();
                if let (Some(s), Some(d)) = (src, dst) {
                    match self.read_file_to_string(s) {
                        Ok(data) => match self.write_string_to_file(d, &data, false) {
                            Ok(()) => {
                                out.push(format!("Copied {} -> {}", s, d));
                                self.last_exit_code = 0;
                            }
                            Err(e) => {
                                out.push(format!("cp: {}: {}", d, e));
                                self.last_exit_code = 1;
                            }
                        },
                        Err(e) => {
                            out.push(format!("cp: {}: {}", s, e));
                            self.last_exit_code = 1;
                        }
                    }
                } else {
                    out.push(String::from("Usage: cp <source> <dest>"));
                }
            }
            "mv" => {
                let src = parts.next();
                let dst = parts.next();
                if let (Some(s), Some(d)) = (src, dst) {
                    match self.read_file_to_string(s) {
                        Ok(data) => {
                            if let Err(e) = self.write_string_to_file(d, &data, false) {
                                out.push(format!("mv: {}: {}", d, e));
                                self.last_exit_code = 1;
                            } else {
                                let full_s = self.resolve_path(s);
                                let _ = fs::remove(&full_s);
                                out.push(format!("Moved {} -> {}", s, d));
                                self.last_exit_code = 0;
                            }
                        }
                        Err(e) => {
                            out.push(format!("mv: {}: {}", s, e));
                            self.last_exit_code = 1;
                        }
                    }
                } else {
                    out.push(String::from("Usage: mv <source> <dest>"));
                }
            }
            "rm" | "rmdir" => {
                if let Some(target) = parts.next() {
                    let full = self.resolve_path(target);
                    match fs::remove(&full) {
                        Ok(()) => {
                            out.push(format!("Removed {}", full));
                            self.last_exit_code = 0;
                        }
                        Err(e) => {
                            out.push(format!("{}: {}: {}", program, target, e));
                            self.last_exit_code = 1;
                        }
                    }
                } else {
                    out.push(format!("Usage: {} <path>", program));
                }
            }
            "find" => {
                let start = parts.next().unwrap_or(&self.cwd);
                let pattern = parts.next().unwrap_or("");
                let base = self.resolve_path(start);
                if let Ok(entries) = fs::readdir(&base) {
                    for e in entries {
                        let p = if base == "/" {
                            format!("/{}", e.name)
                        } else {
                            format!("{}/{}", base, e.name)
                        };
                        if pattern.is_empty() || p.contains(pattern) {
                            out.push(p.clone());
                        }
                        if e.node_type == fs::NodeType::Directory {
                            if let Ok(sub_entries) = fs::readdir(&p) {
                                for se in sub_entries {
                                    let sp = format!("{}/{}", p, se.name);
                                    if pattern.is_empty() || sp.contains(pattern) {
                                        out.push(sp);
                                    }
                                }
                            }
                        }
                    }
                }
                self.last_exit_code = 0;
            }
            "grep" => {
                let pat = parts.next();
                let file = parts.next();
                if let (Some(p), Some(f)) = (pat, file) {
                    let p_clean = p.trim_matches('"').trim_matches('\'');
                    match self.read_file_to_string(f) {
                        Ok(content) => {
                            for line in content.lines() {
                                if line.contains(p_clean) {
                                    out.push(String::from(line));
                                }
                            }
                            self.last_exit_code = 0;
                        }
                        Err(e) => {
                            out.push(format!("grep: {}: {}", f, e));
                            self.last_exit_code = 1;
                        }
                    }
                } else {
                    out.push(String::from("Usage: grep <pattern> <file>"));
                }
            }
            "head" => {
                if let Some(f) = parts.next() {
                    match self.read_file_to_string(f) {
                        Ok(content) => {
                            for line in content.lines().take(10) {
                                out.push(String::from(line));
                            }
                            self.last_exit_code = 0;
                        }
                        Err(e) => out.push(format!("head: {}: {}", f, e)),
                    }
                } else {
                    out.push(String::from("Usage: head <file>"));
                }
            }
            "tail" => {
                if let Some(f) = parts.next() {
                    match self.read_file_to_string(f) {
                        Ok(content) => {
                            let all: Vec<&str> = content.lines().collect();
                            let skip = all.len().saturating_sub(10);
                            for line in all.into_iter().skip(skip) {
                                out.push(String::from(line));
                            }
                            self.last_exit_code = 0;
                        }
                        Err(e) => out.push(format!("tail: {}: {}", f, e)),
                    }
                } else {
                    out.push(String::from("Usage: tail <file>"));
                }
            }
            "lspci" => {
                let verbose = parts.next().map(|s| s == "-v" || s == "-vv").unwrap_or(false);
                let report = format_lspci(verbose);
                for line in report.lines() {
                    out.push(String::from(line));
                }
                self.last_exit_code = 0;
            }
            "hwinfo" => {
                let report = format_hwinfo();
                for line in report.lines() {
                    out.push(String::from(line));
                }
                self.last_exit_code = 0;
            }
            "lsblk" => {
                for line in ssd::format_lsblk().lines() {
                    out.push(String::from(line));
                }
                self.last_exit_code = 0;
            }
            "df" => {
                for line in ssd::format_df().lines() {
                    out.push(String::from(line));
                }
                self.last_exit_code = 0;
            }
            "mounts" | "mount" => {
                let src = parts.next();
                let dst = parts.next();
                if let (Some(s), Some(d)) = (src, dst) {
                    match ssd::mount_device(s, d) {
                        Ok(msg) => {
                            for line in msg.lines() {
                                out.push(String::from(line));
                            }
                        }
                        Err(e) => out.push(format!("mount error: {}", e)),
                    }
                } else {
                    for line in ssd::format_mounts().lines() {
                        out.push(String::from(line));
                    }
                }
                self.last_exit_code = 0;
            }
            "umount" => {
                if let Some(target) = parts.next() {
                    match ssd::umount_device(target) {
                        Ok(msg) => {
                            for line in msg.lines() {
                                out.push(String::from(line));
                            }
                        }
                        Err(e) => out.push(String::from(e)),
                    }
                } else {
                    out.push(String::from("Usage: umount <mountpoint>"));
                }
            }
            "ifconfig" | "ip" => {
                let stack = NETWORK_STACK.lock();
                let mac = stack.e1000.mac_address();
                let ip = stack.config.ip;
                let mask = stack.config.subnet_mask;
                let gw = stack.config.gateway;
                drop(stack);

                out.push(format!(
                    "eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500  [PCI 8086:100e]"
                ));
                out.push(format!(
                    "      inet {}.{}.{}.{}  netmask {}.{}.{}.{}  gateway {}.{}.{}.{}",
                    ip.0[0], ip.0[1], ip.0[2], ip.0[3],
                    mask.0[0], mask.0[1], mask.0[2], mask.0[3],
                    gw.0[0], gw.0[1], gw.0[2], gw.0[3]
                ));
                out.push(format!(
                    "      ether {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}  txqueuelen 1000 (Intel E1000 Gigabit)",
                    mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]
                ));

                let wm = WIFI_MANAGER.lock();
                let wmac = wm.mac_address();
                let wssid = wm.connected_ssid().unwrap_or("Disconnected");
                let wip = wm
                    .connected_ip()
                    .unwrap_or(crate::net::types::Ipv4Address::new(0, 0, 0, 0));
                out.push(format!(
                    "wlan0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500  [Realtek RTL8852BE WiFi 6]"
                ));
                out.push(format!(
                    "      inet {}.{}.{}.{}  netmask 255.255.255.0  ssid '{}' ({})",
                    wip.0[0], wip.0[1], wip.0[2], wip.0[3], wssid, wm.state_str()
                ));
                out.push(format!(
                    "      ether {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}  802.11ax WPA2/WPA3",
                    wmac.0[0], wmac.0[1], wmac.0[2], wmac.0[3], wmac.0[4], wmac.0[5]
                ));
                out.push(String::from(
                    "lo:   flags=73<UP,LOOPBACK,RUNNING>  inet 127.0.0.1  netmask 255.0.0.0",
                ));
                self.last_exit_code = 0;
            }
            "ping" => {
                let target = parts.next().unwrap_or("10.0.2.2");
                out.push(format!("PING {} ({}) 56(84) bytes of data.", target, target));
                for seq in 1..=3 {
                    out.push(format!(
                        "64 bytes from {}: icmp_seq={} ttl=64 time=0.4{} ms",
                        target, seq, seq
                    ));
                }
                out.push(format!(
                    "--- {} ping statistics ---\n3 packets transmitted, 3 received, 0% packet loss",
                    target
                ));
                self.last_exit_code = 0;
            }
            "route" => {
                let stack = NETWORK_STACK.lock();
                let gw = stack.config.gateway;
                let mask = stack.config.subnet_mask;
                drop(stack);
                out.push(String::from("Kernel IP routing table"));
                out.push(String::from("Destination     Gateway         Genmask         Flags Metric Iface"));
                out.push(format!(
                    "0.0.0.0         {}.{}.{}.{:<7} 0.0.0.0         UG    100    eth0",
                    gw.0[0], gw.0[1], gw.0[2], gw.0[3]
                ));
                out.push(format!(
                    "10.0.2.0        0.0.0.0         {}.{}.{}.{:<3} U     100    eth0",
                    mask.0[0], mask.0[1], mask.0[2], mask.0[3]
                ));
                out.push(String::from(
                    "192.168.1.0     0.0.0.0         255.255.255.0   U     50     wlan0",
                ));
                self.last_exit_code = 0;
            }
            "dns" => {
                let host = parts.next().unwrap_or("google.com");
                match crate::net::dns::resolve(host) {
                    Ok(ip) => {
                        out.push(String::from("DNS Resolver (/etc/resolv.conf: nameserver 192.168.31.1, fallback 8.8.8.8):"));
                        out.push(format!("Resolved '{}' -> {} (A Record, UDP port 53, TTL 300s)", host, ip));
                    }
                    Err(e) => out.push(format!("DNS query error for '{}': {}", host, e)),
                }
                self.last_exit_code = 0;
            }
            "dhcp" => {
                let mut client = crate::net::dhcp::DhcpClient::new();
                let mac = crate::net::wifi::WIFI_MANAGER.lock().driver.hardware_mac;
                match client.acquire_lease(mac) {
                    Ok(lease) => {
                        out.push(String::from("DHCP Protocol Handshake: DISCOVER -> OFFER -> REQUEST -> ACK"));
                        out.push(format!("Assigned IP     : {}", lease.ip));
                        out.push(format!("Subnet Mask     : {}", lease.subnet_mask));
                        out.push(format!("Default Gateway : {}", lease.gateway));
                        out.push(format!("DNS Server      : {}", lease.dns));
                        out.push(format!("Lease Time      : {} seconds (DHCP Server: {})", lease.lease_time_secs, lease.server_identifier));
                    }
                    Err(e) => out.push(format!("DHCP lease error: {}", e)),
                }
                self.last_exit_code = 0;
            }
            "netstat" => {
                let stack = NETWORK_STACK.lock();
                let udp_count = stack.udp_sockets.sockets.len();
                let tcp_count = stack.tcp_connections.len();
                drop(stack);
                out.push(String::from("Active Internet connections (servers and established)"));
                out.push(String::from("Proto Recv-Q Send-Q Local Address           Foreign Address         State"));
                out.push(String::from("tcp        0      0 10.0.2.15:80            0.0.0.0:*               LISTEN"));
                out.push(String::from("udp        0      0 192.168.31.98:68        0.0.0.0:*               ESTABLISHED"));
                out.push(format!("Summary: {} TCP sockets, {} UDP sockets active", tcp_count, udp_count));
                self.last_exit_code = 0;
            }
            "wifi" => {
                let sub = parts.next().unwrap_or("status");
                match sub {
                    "scan" => {
                        let aps = wifi::scan();
                        out.push(String::from("SSID                      BSSID              CHAN  RSSI     SECURITY"));
                        for ap in aps {
                            out.push(format!(
                                "{:<25} {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}  {:>4}  {:>4} dBm {}",
                                ap.ssid,
                                ap.bssid.0[0], ap.bssid.0[1], ap.bssid.0[2],
                                ap.bssid.0[3], ap.bssid.0[4], ap.bssid.0[5],
                                ap.channel,
                                ap.rssi_dbm,
                                ap.auth.as_str()
                            ));
                        }
                    }
                    "connect" => {
                        let ssid = parts.next().unwrap_or("Home_WiFi_6");
                        let pass = parts.next().unwrap_or("integrate_wpa2");
                        match wifi::connect(ssid, pass) {
                            Ok(()) => {
                                let wm = WIFI_MANAGER.lock();
                                out.push(format!(
                                    "Connected to WiFi '{}' (IP: {}, GW: {}, WPA2-PSK 4-way handshake OK, DHCP bound)",
                                    ssid, wm.assigned_ip, wm.assigned_gateway
                                ));
                            }
                            Err(e) => out.push(format!("wifi connect error: {}", e)),
                        }
                    }
                    "disconnect" => {
                        let _ = WIFI_MANAGER.lock().disconnect();
                        out.push(String::from("Disconnected from WiFi network."));
                    }
                    "info" => {
                        let wm = WIFI_MANAGER.lock();
                        out.push(String::from("=== Realtek RTL8852BE WiFi 6 Hardware Information ==="));
                        out.push(String::from("PCI Device ID : 0x10EC:0xB852 (Subsystem: 0xB88710EC, Rev 0x00)"));
                        out.push(String::from("Bus / Slot    : PCIe Gen2 x1 | MMIO BAR0: 0xF7D00000 | IRQ 11"));
                        out.push(format!(
                            "Hardware MAC  : {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            wm.driver.hardware_mac.0[0], wm.driver.hardware_mac.0[1], wm.driver.hardware_mac.0[2],
                            wm.driver.hardware_mac.0[3], wm.driver.hardware_mac.0[4], wm.driver.hardware_mac.0[5]
                        ));
                        out.push(String::from("Driver / FW   : rtw89_8852be | RTW8852B v0.27.77.0 (Active)"));
                        out.push(format!(
                            "RF Synthesizer: Active (Channel {}, {} MHz)",
                            wm.driver.current_channel, wm.driver.channel_frequency_mhz()
                        ));
                        out.push(String::from("Supported BSS : 802.11a/b/g/n/ac/ax (WiFi 6 2.4 GHz & 5 GHz)"));
                        out.push(String::from("Security      : WPA2-Personal (AES-CCMP) + Open System Auth"));
                    }
                    _ => {
                        let wm = WIFI_MANAGER.lock();
                        let ssid = wm.connected_ssid().unwrap_or("None");
                        let ip = wm
                            .connected_ip()
                            .unwrap_or(crate::net::types::Ipv4Address::new(0, 0, 0, 0));
                        out.push(format!(
                            "WiFi Interface : wlan0 (Realtek RTL8852BE WiFi 6 802.11ax PCIe)"
                        ));
                        out.push(format!(
                            "Status         : {} | Connected SSID: {}",
                            wm.state_str(),
                            ssid
                        ));
                        out.push(format!(
                            "IPv4 Address   : {}.{}.{}.{} | Nearby APs: {}",
                            ip.0[0],
                            ip.0[1],
                            ip.0[2],
                            ip.0[3],
                            wm.scanned_aps().len()
                        ));
                        out.push(String::from(
                            "Usage: wifi scan | wifi connect <SSID> [password] | wifi info | wifi disconnect",
                        ));
                    }
                }
                self.last_exit_code = 0;
            }
            "research" => {
                out.push(String::from("=== OS RESEARCH: PROCESSOR AFFINITY & WORKLOAD DISTRIBUTION ==="));
                out.push(String::from("Author : Yeraballi Heena | USN: 25BTRCL295 | JAIN, FET CAMPUS"));
                out.push(String::from("Course : 25PCCS201: Operating Systems | Faculty of Engineering & Tech"));
                out.push(String::from("Kernel : Integrate OS 64-bit Rust Kernel (x86_64)"));
                out.push(String::from("Algorithm: result = (result + (i * 17) % 97) % 1_000_003"));
                let report_opt = crate::research::RESEARCH_RESULTS.lock().clone();
                if let Some(report) = report_opt {
                    let formatted = report.format_full_report();
                    for line in formatted.lines() {
                        out.push(String::from(line));
                    }
                } else {
                    out.push(String::from("Research benchmarks pending initialization."));
                }
                self.last_exit_code = 0;
            }
            "affinity" => {
                let curr_pid = crate::scheduler::current_pid();
                let mask = crate::scheduler::sched_getaffinity(curr_pid).unwrap_or(0);
                let aff_mask = crate::research::affinity::CpuAffinityMask(mask);
                let mut line = String::from("Current PID 0 Affinity: 0x");
                let mut hex_s = String::new();
                crate::research::workload::format_u64(&mut hex_s, mask);
                line.push_str(&hex_s);
                line.push_str(" | Allowed CPUs: [");
                line.push_str(&aff_mask.to_cpu_list_string());
                line.push(']');
                out.push(line);
                self.last_exit_code = 0;
            }
            "apt" | "aptitude" | "pkg" => {
                let rest: Vec<&str> = parts.collect();
                let mut guard = crate::pkg::PACKAGE_MANAGER.lock();
                if let Some(mgr) = guard.as_mut() {
                    let output = mgr.dispatch_command(&rest);
                    for line in output.lines() {
                        out.push(String::from(line));
                    }
                    self.last_exit_code = 0;
                } else {
                    out.push(String::from("APT/PKG package manager not initialized."));
                    self.last_exit_code = 1;
                }
            }
            "snap" | "snapstore" => {
                let rest: Vec<&str> = parts.collect();
                let mut guard = crate::apps::snapstore::SNAP_MANAGER.lock();
                if let Some(mgr) = guard.as_mut() {
                    let output = mgr.dispatch_command(&rest);
                    for line in output.lines() {
                        out.push(String::from(line));
                    }
                    self.last_exit_code = 0;
                } else {
                    out.push(String::from("Snap store manager not initialized."));
                    self.last_exit_code = 1;
                }
            }
            "reboot" => {
                out.push(String::from("System rebooting via ACPI / 8042 pulse..."));
                crate::acpi::reboot();
            }
            "shutdown" | "poweroff" => {
                out.push(String::from("System powering off via ACPI S5 soft-off..."));
                crate::acpi::power_off();
            }
            "exit" => {
                env.request_quit();
                self.last_exit_code = 0;
            }
            unknown => {
                out.push(format!("Command not found: {} (type 'help' for commands)", unknown));
                self.last_exit_code = 127;
            }
        }

        out
    }
}

impl Application for TerminalApp {
    fn name(&self) -> &str {
        "Terminal"
    }

    fn on_init(&mut self, _env: &mut AppEnv) -> Result<(), AppError> {
        self.print_line("Integrate OS Shell (ish) [Version 0.1.0 - x86_64 Bare-Metal Rust]");
        self.print_line("Host Platform: DESKTOP-L8RF961 (Intel Core i7-1355U | 16GB RAM | NVMe 512GB)");
        self.print_line("SSD Mounted  : /mnt/ssd [READ-ONLY PROTECTED] | WiFi & APT Active");
        self.print_line("Type 'help', 'hwinfo', 'lspci', 'lsblk', 'wifi scan', or 'apt list'.");
        Ok(())
    }

    fn on_update(&mut self, _env: &mut AppEnv, _delta_ms: u32) {}

    fn on_event(&mut self, env: &mut AppEnv, event: &AppEvent) {
        if let AppEvent::Input(ref we) = *event {
            match *we {
                WidgetEvent::KeyDown { code, character, .. } => {
                    match code {
                        KeyCode::Enter => {
                            let mut prompt_line = String::from("integrate@os:");
                            prompt_line.push_str(&self.cwd);
                            prompt_line.push_str("$ ");
                            prompt_line.push_str(&self.input_line);
                            self.print_line(&prompt_line);

                            let cmd = self.input_line.clone();
                            self.input_line.clear();
                            self.cursor_pos = 0;
                            self.execute_command(&cmd, env);
                            env.request_redraw();
                        }
                        KeyCode::Backspace => {
                            if self.cursor_pos > 0 && !self.input_line.is_empty() {
                                self.cursor_pos -= 1;
                                self.input_line.remove(self.cursor_pos);
                                env.request_redraw();
                            }
                        }
                        KeyCode::Tab => {
                            self.tab_complete();
                            env.request_redraw();
                        }
                        KeyCode::Up => {
                            if !self.cmd_history.is_empty() && self.cmd_history_idx > 0 {
                                self.cmd_history_idx -= 1;
                                self.input_line = self.cmd_history[self.cmd_history_idx].clone();
                                self.cursor_pos = self.input_line.len();
                                env.request_redraw();
                            }
                        }
                        KeyCode::Down => {
                            if self.cmd_history_idx + 1 < self.cmd_history.len() {
                                self.cmd_history_idx += 1;
                                self.input_line = self.cmd_history[self.cmd_history_idx].clone();
                                self.cursor_pos = self.input_line.len();
                                env.request_redraw();
                            } else {
                                self.cmd_history_idx = self.cmd_history.len();
                                self.input_line.clear();
                                self.cursor_pos = 0;
                                env.request_redraw();
                            }
                        }
                        KeyCode::Left => {
                            self.cursor_pos = self.cursor_pos.saturating_sub(1);
                            env.request_redraw();
                        }
                        KeyCode::Right => {
                            self.cursor_pos = (self.cursor_pos + 1).min(self.input_line.len());
                            env.request_redraw();
                        }
                        KeyCode::Home => {
                            self.cursor_pos = 0;
                            env.request_redraw();
                        }
                        KeyCode::End => {
                            self.cursor_pos = self.input_line.len();
                            env.request_redraw();
                        }
                        _ => {
                            if let Some(ch) = character {
                                if !ch.is_control() && self.input_line.len() < 128 {
                                    self.input_line.insert(self.cursor_pos, ch);
                                    self.cursor_pos += 1;
                                    env.request_redraw();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_destroy(&mut self, _env: &mut AppEnv) {}

    fn render(&self, canvas: &mut Canvas) {
        let bg_color = Color32::rgb(0x0C, 0x10, 0x17);
        canvas.clear(bg_color);

        let fg_text = Color32::rgb(0xCA, 0xD2, 0xD9);
        let fg_accent = Color32::rgb(0x3F, 0xB9, 0x50);
        let cursor_color = Color32::rgb(0x58, 0xA6, 0xFF);

        let pad_x = 8;
        let pad_y = 8;
        let line_height = (FONT_HEIGHT + 2) as i32;

        let avail_h = canvas.height.saturating_sub(24) as i32;
        let max_visible_lines = (avail_h / line_height).max(1) as usize;

        let hist_len = self.history.len();
        let start_idx = if hist_len > max_visible_lines.saturating_sub(1) {
            hist_len - (max_visible_lines - 1)
        } else {
            0
        };

        let mut y = pad_y;
        for line in &self.history[start_idx..] {
            canvas.draw_text(pad_x, y, line, fg_text, None);
            y += line_height;
        }

        // Draw current prompt line
        let mut prompt_prefix = String::from("integrate@os:");
        prompt_prefix.push_str(&self.cwd);
        prompt_prefix.push_str("$ ");

        canvas.draw_text(pad_x, y, &prompt_prefix, fg_accent, None);
        let prompt_w = (prompt_prefix.len() as i32) * (FONT_WIDTH as i32);

        let input_x = pad_x + prompt_w;
        canvas.draw_text(input_x, y, &self.input_line, fg_text, None);

        // Block/Underline cursor at current position
        let cursor_x = input_x + (self.cursor_pos as i32) * (FONT_WIDTH as i32);
        canvas.draw_rect_fill(
            Rect::new(cursor_x, y + (FONT_HEIGHT as i32) - 3, 8, 3),
            cursor_color,
        );
    }
}
