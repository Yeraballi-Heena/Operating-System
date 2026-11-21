//! Integrate OS — Dual-Sink Kernel Console & Structured Logger
//!
//! Mirror-writes all kernel console output and structured logs to both:
//! 1. VGA Text Console (`0xB8000`) for live VirtualBox display visibility
//! 2. `COM1` Serial UART (`0x3F8`) for host file capture (`build/serial.log`)

use crate::serial::SerialWriter;
use crate::vga::{self, Color, VgaWriter};
use core::fmt::{self, Write};

struct DualConsoleWriter;

impl Write for DualConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = SerialWriter.write_str(s);
        let _ = VgaWriter.write_str(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _kprint(args: fmt::Arguments<'_>) {
    let _ = DualConsoleWriter.write_fmt(args);
}

#[doc(hidden)]
pub fn _kprint_colored(fg: Color, bg: Color, args: fmt::Arguments<'_>) {
    vga::set_color(fg, bg);
    let _ = DualConsoleWriter.write_fmt(args);
    vga::set_color(Color::LightGray, Color::Black);
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::logger::_kprint(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! kprintln {
    () => {
        $crate::kprint!("\n")
    };
    ($($arg:tt)*) => {
        $crate::kprint!("{}\n", core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! klog_info {
    ($($arg:tt)*) => {{
        $crate::logger::_kprint_colored(
            $crate::vga::Color::LightCyan,
            $crate::vga::Color::Black,
            core::format_args!("[INFO] ")
        );
        $crate::kprintln!($($arg)*);
    }};
}

#[macro_export]
macro_rules! klog_ok {
    ($($arg:tt)*) => {{
        $crate::logger::_kprint_colored(
            $crate::vga::Color::LightGreen,
            $crate::vga::Color::Black,
            core::format_args!("[ OK ] ")
        );
        $crate::kprintln!($($arg)*);
    }};
}

#[macro_export]
macro_rules! klog_excp {
    ($($arg:tt)*) => {{
        $crate::logger::_kprint_colored(
            $crate::vga::Color::Yellow,
            $crate::vga::Color::Black,
            core::format_args!("[EXCP] ")
        );
        $crate::kprintln!($($arg)*);
    }};
}
