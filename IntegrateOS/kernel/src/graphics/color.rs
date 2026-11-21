//! 32-Bit TrueColor RGB / RGBA Pixel Format & Alpha Compositing
//!
//! Provides `Color32` representing 32-bit ARGB/XRGB (8-bit per channel)
//! color with alpha blending, linear interpolation (gradients), and predefined palettes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Color32 {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl Color32 {
    /// Creates an opaque color from red, green, and blue components.
    #[inline(always)]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Creates a color with custom alpha transparency.
    #[inline(always)]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Decodes a 32-bit `0xAARRGGBB` integer into `Color32`.
    #[inline(always)]
    pub const fn from_u32(val: u32) -> Self {
        let a = ((val >> 24) & 0xFF) as u8;
        let r = ((val >> 16) & 0xFF) as u8;
        let g = ((val >> 8) & 0xFF) as u8;
        let b = (val & 0xFF) as u8;
        Self { r, g, b, a }
    }

    /// Decodes a 24-bit `0x00RRGGBB` integer into an opaque `Color32`.
    #[inline(always)]
    pub const fn from_rgb_u32(val: u32) -> Self {
        let r = ((val >> 16) & 0xFF) as u8;
        let g = ((val >> 8) & 0xFF) as u8;
        let b = (val & 0xFF) as u8;
        Self { r, g, b, a: 255 }
    }

    /// Encodes this color into a 32-bit `0xAARRGGBB` integer.
    #[inline(always)]
    pub const fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24)
            | ((self.r as u32) << 16)
            | ((self.g as u32) << 8)
            | (self.b as u32)
    }

    /// Returns the raw 32-bit little-endian value for hardware linear framebuffer writes.
    ///
    /// On little-endian x86, bytes in memory are ordered `[B, G, R, A]`, matching
    /// standard 32bpp linear framebuffers.
    #[inline(always)]
    pub const fn to_raw(&self) -> u32 {
        ((self.a as u32) << 24)
            | ((self.r as u32) << 16)
            | ((self.g as u32) << 8)
            | (self.b as u32)
    }

    /// Alpha-blends this color (source) over a background color (destination)
    /// using standard Porter-Duff source-over compositing.
    #[inline]
    pub fn alpha_blend(&self, bg: Color32) -> Color32 {
        if self.a == 255 {
            return *self;
        }
        if self.a == 0 {
            return bg;
        }

        let alpha = self.a as u32;
        let inv_alpha = 255 - alpha;

        let r = ((self.r as u32 * alpha) + (bg.r as u32 * inv_alpha)) / 255;
        let g = ((self.g as u32 * alpha) + (bg.g as u32 * inv_alpha)) / 255;
        let b = ((self.b as u32 * alpha) + (bg.b as u32 * inv_alpha)) / 255;

        Color32::rgba(r as u8, g as u8, b as u8, 255)
    }

    /// Linearly interpolates between `c1` and `c2` by fraction `t_num / t_den`.
    #[inline]
    pub fn lerp(c1: Color32, c2: Color32, t_num: u32, t_den: u32) -> Color32 {
        if t_den == 0 || t_num == 0 {
            return c1;
        }
        if t_num >= t_den {
            return c2;
        }

        let inv = t_den - t_num;
        let r = ((c1.r as u32 * inv) + (c2.r as u32 * t_num)) / t_den;
        let g = ((c1.g as u32 * inv) + (c2.g as u32 * t_num)) / t_den;
        let b = ((c1.b as u32 * inv) + (c2.b as u32 * t_num)) / t_den;
        let a = ((c1.a as u32 * inv) + (c2.a as u32 * t_num)) / t_den;

        Color32::rgba(r as u8, g as u8, b as u8, a as u8)
    }

    // Predefined classic colors
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const CYAN: Self = Self::rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    pub const LIGHT_GRAY: Self = Self::rgb(192, 192, 192);
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    pub const DARK_GRAY: Self = Self::rgb(64, 64, 64);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    // Modern Integrate OS UI Palette
    pub const BG_DEEP: Self = Self::rgb(10, 15, 29);         // 0x0A0F1D
    pub const BG_SLATE: Self = Self::rgb(30, 34, 42);        // 0x1E222A
    pub const PANEL_BG: Self = Self::rgb(24, 27, 33);        // 0x181B21
    pub const PANEL_BORDER: Self = Self::rgb(45, 55, 72);    // 0x2D3748
    pub const ACCENT_BLUE: Self = Self::rgb(56, 189, 248);   // 0x38BDF8
    pub const ACCENT_ROYAL: Self = Self::rgb(37, 99, 235);   // 0x2563EB
    pub const EMERALD_GREEN: Self = Self::rgb(16, 185, 129); // 0x10B981
    pub const AMBER_ORANGE: Self = Self::rgb(245, 158, 11);  // 0xF59E0B
    pub const CORAL_RED: Self = Self::rgb(239, 68, 68);      // 0xEF4444
    pub const TEXT_PRIMARY: Self = Self::rgb(248, 250, 252); // 0xF8FAFC
    pub const TEXT_MUTED: Self = Self::rgb(148, 163, 184);   // 0x94A3B8
}
