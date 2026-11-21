//! Integrate OS — Time Subsystem Types & Calendar Primitives
//!
//! Provides Gregorian calendar computations, ISO-8601 formatting, Unix timestamp
//! conversion, Day-of-Week evaluation, and duration representation.

use alloc::string::String;
use alloc::format;
use core::fmt;

/// Day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

impl Weekday {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sunday => "Sunday",
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
        }
    }

    pub const fn short_str(self) -> &'static str {
        match self {
            Self::Sunday => "Sun",
            Self::Monday => "Mon",
            Self::Tuesday => "Tue",
            Self::Wednesday => "Wed",
            Self::Thursday => "Thu",
            Self::Friday => "Fri",
            Self::Saturday => "Sat",
        }
    }
}

/// Calendar Month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl Month {
    pub const fn from_u8(m: u8) -> Option<Self> {
        match m {
            1 => Some(Self::January),
            2 => Some(Self::February),
            3 => Some(Self::March),
            4 => Some(Self::April),
            5 => Some(Self::May),
            6 => Some(Self::June),
            7 => Some(Self::July),
            8 => Some(Self::August),
            9 => Some(Self::September),
            10 => Some(Self::October),
            11 => Some(Self::November),
            12 => Some(Self::December),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::January => "January",
            Self::February => "February",
            Self::March => "March",
            Self::April => "April",
            Self::May => "May",
            Self::June => "June",
            Self::July => "July",
            Self::August => "August",
            Self::September => "September",
            Self::October => "October",
            Self::November => "November",
            Self::December => "December",
        }
    }

    pub const fn short_str(self) -> &'static str {
        match self {
            Self::January => "Jan",
            Self::February => "Feb",
            Self::March => "Mar",
            Self::April => "Apr",
            Self::May => "May",
            Self::June => "Jun",
            Self::July => "Jul",
            Self::August => "Aug",
            Self::September => "Sep",
            Self::October => "Oct",
            Self::November => "Nov",
            Self::December => "Dec",
        }
    }
}

/// Complete Gregorian Date and Time representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    /// Epoch baseline: 1970-01-01 00:00:00 UTC.
    pub const UNIX_EPOCH: Self = Self {
        year: 1970,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
    };

    pub const fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Determines if a Gregorian year is a leap year.
    #[inline]
    pub const fn is_leap_year(year: u16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    /// Number of days in the specified month of a given year.
    pub const fn days_in_month(year: u16, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }

    /// Validates date and time fields.
    pub const fn is_valid(&self) -> bool {
        if self.year < 1970 || self.month < 1 || self.month > 12 {
            return false;
        }
        let max_days = Self::days_in_month(self.year, self.month);
        if self.day < 1 || self.day > max_days {
            return false;
        }
        if self.hour > 23 || self.minute > 59 || self.second > 59 {
            return false;
        }
        true
    }

    /// Calculates the day of the week using Sakamoto's algorithm.
    pub const fn weekday(&self) -> Weekday {
        let mut y = self.year as i32;
        let m = self.month as i32;
        let d = self.day as i32;

        const TABLE: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        if m < 3 {
            y -= 1;
        }
        let day_idx = (y + y / 4 - y / 100 + y / 400 + TABLE[(m - 1) as usize] + d) % 7;
        match day_idx {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }

    /// Converts this `DateTime` into seconds since the Unix Epoch (1970-01-01 00:00:00 UTC).
    pub fn to_unix_timestamp(&self) -> u64 {
        let mut days: u64 = 0;

        // Days for intervening full years
        for y in 1970..self.year {
            days += if Self::is_leap_year(y) { 366 } else { 365 };
        }

        // Days for elapsed months in current year
        for m in 1..self.month {
            days += Self::days_in_month(self.year, m) as u64;
        }

        // Days in current month (1-based to 0-based)
        days += (self.day.saturating_sub(1)) as u64;

        let total_seconds = days * 86400
            + (self.hour as u64) * 3600
            + (self.minute as u64) * 60
            + (self.second as u64);

        total_seconds
    }

    /// Reconstructs a `DateTime` from a 64-bit Unix timestamp.
    pub fn from_unix_timestamp(ts: u64) -> Self {
        let mut seconds_remaining = ts;
        let second = (seconds_remaining % 60) as u8;
        seconds_remaining /= 60;
        let minute = (seconds_remaining % 60) as u8;
        seconds_remaining /= 60;
        let hour = (seconds_remaining % 24) as u8;
        let mut days_remaining = seconds_remaining / 24;

        let mut year: u16 = 1970;
        loop {
            let year_days = if Self::is_leap_year(year) { 366 } else { 365 };
            if days_remaining >= year_days {
                days_remaining -= year_days;
                year += 1;
            } else {
                break;
            }
        }

        let mut month: u8 = 1;
        while month <= 12 {
            let dim = Self::days_in_month(year, month) as u64;
            if days_remaining >= dim {
                days_remaining -= dim;
                month += 1;
            } else {
                break;
            }
        }

        let day = (days_remaining + 1) as u8;

        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Formats as standard ISO-8601 string: `"YYYY-MM-DDTHH:MM:SSZ"`.
    pub fn format_iso8601(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Formats as human-readable string: `"YYYY-MM-DD HH:MM:SS"`.
    pub fn format_human(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Duration representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    micros: u64,
}

impl Duration {
    pub const ZERO: Self = Self { micros: 0 };

    pub const fn from_micros(micros: u64) -> Self {
        Self { micros }
    }

    pub const fn from_millis(millis: u64) -> Self {
        Self {
            micros: millis * 1000,
        }
    }

    pub const fn from_secs(secs: u64) -> Self {
        Self {
            micros: secs * 1_000_000,
        }
    }

    #[inline]
    pub const fn as_micros(self) -> u64 {
        self.micros
    }

    #[inline]
    pub const fn as_millis(self) -> u64 {
        self.micros / 1000
    }

    #[inline]
    pub const fn as_secs(self) -> u64 {
        self.micros / 1_000_000
    }
}

/// Time subsystem error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    RtcNotReady,
    InvalidDateTime,
    BatteryDepleted,
    TimerNotFound,
    TimerLimitReached,
    HardwareFault,
}

impl TimeError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RtcNotReady => "RTC hardware update in progress / busy",
            Self::InvalidDateTime => "Invalid calendar date or time parameters",
            Self::BatteryDepleted => "CMOS battery depleted or invalid checksum",
            Self::TimerNotFound => "Timer ID not found in active timer registry",
            Self::TimerLimitReached => "Maximum timer capacity reached",
            Self::HardwareFault => "Hardware timer / RTC failure",
        }
    }
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
