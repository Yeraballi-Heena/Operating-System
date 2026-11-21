//! Integrate OS — Motorola MC146818 Compatible CMOS Real-Time Clock (RTC) Driver
//!
//! Provides atomic, debounce-verified reading of date, time, and status registers
//! from the motherboard CMOS RTC ports 0x70 / 0x71.

use crate::arch::x86_64::io::{inb, outb};
use super::types::{DateTime, TimeError};

pub const CMOS_ADDR_PORT: u16 = 0x70;
pub const CMOS_DATA_PORT: u16 = 0x71;

pub const RTC_SECONDS: u8 = 0x00;
pub const RTC_MINUTES: u8 = 0x02;
pub const RTC_HOURS: u8 = 0x04;
pub const RTC_DAY_OF_WEEK: u8 = 0x06;
pub const RTC_DAY_OF_MONTH: u8 = 0x07;
pub const RTC_MONTH: u8 = 0x08;
pub const RTC_YEAR: u8 = 0x09;
pub const RTC_STATUS_A: u8 = 0x0A;
pub const RTC_STATUS_B: u8 = 0x0B;
pub const RTC_STATUS_C: u8 = 0x0C;
pub const RTC_STATUS_D: u8 = 0x0D;
pub const RTC_CENTURY: u8 = 0x32;

/// Low-level CMOS RTC controller.
pub struct CmosRtc;

impl CmosRtc {
    /// Reads a CMOS register byte safely.
    /// Bit 7 of port 0x70 controls NMI masking; we preserve or keep NMI enabled (0x00..0x7F).
    pub fn read_reg(reg: u8) -> u8 {
        // SAFETY: Standard x86 port I/O to CMOS index and data ports.
        unsafe {
            outb(CMOS_ADDR_PORT, reg & 0x7F);
            inb(CMOS_DATA_PORT)
        }
    }

    /// Writes a value to a CMOS register.
    pub fn write_reg(reg: u8, val: u8) {
        // SAFETY: Standard x86 port I/O to CMOS index and data ports.
        unsafe {
            outb(CMOS_ADDR_PORT, reg & 0x7F);
            outb(CMOS_DATA_PORT, val);
        }
    }

    /// Returns `true` if an RTC update cycle is currently in progress (Status A bit 7).
    pub fn is_update_in_progress() -> bool {
        (Self::read_reg(RTC_STATUS_A) & 0x80) != 0
    }

    /// Checks if the CMOS battery has sufficient power (Status D bit 7).
    pub fn is_battery_healthy() -> bool {
        (Self::read_reg(RTC_STATUS_D) & 0x80) != 0
    }

    /// Converts Binary-Coded Decimal (BCD) to binary.
    #[inline]
    pub const fn bcd_to_binary(val: u8) -> u8 {
        ((val >> 4) * 10) + (val & 0x0F)
    }

    /// Converts binary to Binary-Coded Decimal (BCD).
    #[inline]
    pub const fn binary_to_bcd(val: u8) -> u8 {
        ((val / 10) << 4) | (val % 10)
    }

    /// Reads raw registers into a tuple: (sec, min, hour, day, month, year, century).
    fn read_raw_registers() -> (u8, u8, u8, u8, u8, u8, u8) {
        let sec = Self::read_reg(RTC_SECONDS);
        let min = Self::read_reg(RTC_MINUTES);
        let hour = Self::read_reg(RTC_HOURS);
        let day = Self::read_reg(RTC_DAY_OF_MONTH);
        let month = Self::read_reg(RTC_MONTH);
        let year = Self::read_reg(RTC_YEAR);
        let century = Self::read_reg(RTC_CENTURY);
        (sec, min, hour, day, month, year, century)
    }

    /// Reads the current wall-clock date and time with double-read debouncing.
    pub fn read_datetime() -> Result<DateTime, TimeError> {
        // 1. Wait for Update-In-Progress to clear (up to 100,000 spin loops)
        let mut spins = 0;
        while Self::is_update_in_progress() {
            spins += 1;
            if spins > 100_000 {
                return Err(TimeError::RtcNotReady);
            }
            core::hint::spin_loop();
        }

        // 2. Double-read registers until two consecutive reads match (prevents mid-tick rollover)
        let mut raw = Self::read_raw_registers();
        let mut attempts = 0;
        loop {
            let next_raw = Self::read_raw_registers();
            if raw == next_raw {
                break;
            }
            raw = next_raw;
            attempts += 1;
            if attempts > 20 {
                return Err(TimeError::RtcNotReady);
            }
        }

        let (mut sec, mut min, mut hour, mut day, mut month, mut year, mut century) = raw;

        // 3. Query Status Register B for BCD vs Binary and 12-hour vs 24-hour mode
        let status_b = Self::read_reg(RTC_STATUS_B);
        let is_binary = (status_b & 0x04) != 0;
        let is_24hr = (status_b & 0x02) != 0;

        // 4. Convert BCD to binary if required
        if !is_binary {
            sec = Self::bcd_to_binary(sec);
            min = Self::bcd_to_binary(min);
            hour = ((hour & 0x7F) == hour)
                .then(|| Self::bcd_to_binary(hour))
                .unwrap_or_else(|| Self::bcd_to_binary(hour & 0x7F) | 0x80);
            day = Self::bcd_to_binary(day);
            month = Self::bcd_to_binary(month);
            year = Self::bcd_to_binary(year);
            if century != 0 {
                century = Self::bcd_to_binary(century);
            }
        }

        // 5. Convert 12-hour mode to 24-hour mode if required
        if !is_24hr {
            let is_pm = (hour & 0x80) != 0;
            hour &= 0x7F;
            if is_pm && hour < 12 {
                hour += 12;
            } else if !is_pm && hour == 12 {
                hour = 0;
            }
        }

        // 6. Compute full 4-digit Gregorian year
        let full_year = if century >= 19 && century <= 21 {
            (century as u16) * 100 + (year as u16)
        } else if year < 70 {
            2000 + (year as u16)
        } else {
            1900 + (year as u16)
        };

        let dt = DateTime::new(full_year, month, day, hour, min, sec);
        if dt.is_valid() {
            Ok(dt)
        } else {
            Err(TimeError::InvalidDateTime)
        }
    }
}
