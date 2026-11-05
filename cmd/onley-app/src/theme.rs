//! The black, paper and pink palette.

use nsh_core::element::{AdaptiveColor, rgb, rgba};

/// The same color in both appearances: the client always shows the light one.
const fn tone(hex: u32) -> AdaptiveColor {
    AdaptiveColor {
        light: rgb(hex).to_hsla(),
        dark: rgb(hex).to_hsla(),
    }
}

/// `hex` at `alpha` in both appearances.
const fn tone_a(hex: u32, alpha: f32) -> AdaptiveColor {
    let value = rgba(hex << 8 | (alpha * 255.0 + 0.5) as u32);
    AdaptiveColor {
        light: value.to_hsla(),
        dark: value.to_hsla(),
    }
}

pub const BACKGROUND: AdaptiveColor = tone(0xF4F1EB);
pub const SURFACE: AdaptiveColor = tone(0xFFFFFF);
pub const BORDER: AdaptiveColor = tone(0x111111);
pub const TEXT: AdaptiveColor = tone(0x111111);
pub const MUTED: AdaptiveColor = tone(0x6B6B6B);
pub const ACCENT: AdaptiveColor = tone(0xFF00A0);
pub const ACCENT_SOFT: AdaptiveColor = tone_a(0xFF00A0, 0.12);
pub const SUCCESS: AdaptiveColor = tone(0x1F6B3A);
pub const SUCCESS_SOFT: AdaptiveColor = tone(0xB9E4C9);
pub const DANGER: AdaptiveColor = tone(0xFF00A0);
pub const OVERLAY: AdaptiveColor = tone_a(0x111111, 0.55);

pub const DISPLAY_FONT: &str = "Impact";
pub const MONO_FONT: &str = "Courier New";
