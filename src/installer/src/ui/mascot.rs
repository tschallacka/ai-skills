// MODE: DEV
// PACKAGE: PROD
//! The Minecraft-mascot sprite -- ported from installer/src/05-config.sh's
//! `ART` and 30-render.sh's `detect_color_mode`/`fg_sgr`/`color_for`/
//! `eye_row_for`. Painted as a colored overlay at an absolute terminal
//! position AFTER render.rs's plain-ASCII frame draws, rather than embedded
//! in it: render.rs's cell-width accounting (`pad`/`wrap`, the "every line
//! is exactly `cols`" invariant its tests check) has no notion of an SGR
//! escape's zero display width, so mixing the two would break it. Only the
//! "left-bottom" placement (bash's `IUI_HEAD_PLACE=left-bottom`, an ordinary
//! terminal) is ported -- "right-top" existed to make room for the hint
//! carousel, which mod.rs doesn't have.

use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeState {
    Front,
    Left,
    Right,
}

/// One entry per tick (a tick is one second, mod.rs's idle-tick interval).
/// The dwell at the front is deliberate, same reasoning as install.sh's own
/// comment: a sprite that moves immediately reads as a glitch, not an
/// animation's start.
const EYE_FRAMES: &[EyeState] = &[
    EyeState::Front,
    EyeState::Front,
    EyeState::Front,
    EyeState::Front,
    EyeState::Front,
    EyeState::Front,
    EyeState::Right,
    EyeState::Right,
    EyeState::Front,
    EyeState::Left,
    EyeState::Left,
    EyeState::Front,
    EyeState::Left,
    EyeState::Right,
    EyeState::Front,
];

pub struct EyeAnimator {
    index: usize,
}

impl EyeAnimator {
    pub fn new() -> Self {
        EyeAnimator { index: 0 }
    }

    pub fn current(&self) -> EyeState {
        EYE_FRAMES[self.index]
    }

    pub fn advance(&mut self) {
        self.index = (self.index + 1) % EYE_FRAMES.len();
    }
}

impl Default for EyeAnimator {
    fn default() -> Self {
        Self::new()
    }
}

/// 16 rows of 16 six-hex-digit pixels, one row per entry -- verbatim from
/// installer/src/05-config.sh's ART. Rows 8 and 9 (the eyes) are
/// substituted per frame by `eye_row`.
const ART: [&str; 16] = [
    "f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fdfd5e fcfd5f fcf347 fcf347",
    "f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fbfb5d fdfd5e fcf347 fcf347",
    "e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fddc51 fddc51 fdbb37 fdbb37",
    "e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fcdc51 fcdc51 fdbb37 fdbb37",
    "c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18",
    "c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18",
    "d8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18",
    "d8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18",
    "c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 dd8100 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601",
    "c27f18 c27f18 6c3100 6c3100 c27f18 c27f18 67522d 67522d 67522d 67522d 883300 883300 c27f18 c27f18 6c3100 6c3100",
    "623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000",
    "623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000",
    "280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000",
    "280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000",
    "300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00",
    "300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00",
];

/// Both eye rows carry the same pixels in every state, so one row per state
/// serves rows 8 and 9 both.
fn eye_row(state: EyeState) -> &'static str {
    match state {
        EyeState::Left => "c27f18 c27f18 009c00 009c00 fbfbfb fbfbfb c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601",
        EyeState::Right => "c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 fbfbfb fbfbfb 009c00 009c00 d68601 d68601",
        EyeState::Front => "c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    Ansi8,
    None,
}

/// 24-bit SGR is not universal (macOS Terminal.app has never supported it),
/// so this probes once and lets `fg_sgr` downgrade -- same reasoning as
/// install.sh's `detect_color_mode`.
pub fn detect_color_mode() -> ColorMode {
    if matches!(
        std::env::var("COLORTERM").as_deref(),
        Ok("truecolor") | Ok("24bit")
    ) {
        return ColorMode::TrueColor;
    }
    let colors = tput_colors();
    if colors >= 16_777_216 {
        ColorMode::TrueColor
    } else if colors >= 256 {
        ColorMode::Ansi256
    } else if colors >= 8 {
        ColorMode::Ansi8
    } else {
        ColorMode::None
    }
}

fn tput_colors() -> i64 {
    Command::new("tput")
        .arg("colors")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or(0)
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0);
    (byte(0), byte(2), byte(4))
}

fn fg_sgr(mode: ColorMode, rgb: (u8, u8, u8)) -> String {
    match mode {
        ColorMode::TrueColor => format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2),
        ColorMode::None => String::new(),
        ColorMode::Ansi256 => {
            let scale = |c: u8| c as u32 * 5 / 255;
            let index = 16 + 36 * scale(rgb.0) + 6 * scale(rgb.1) + scale(rgb.2);
            format!("\x1b[38;5;{index}m")
        }
        ColorMode::Ansi8 => {
            let index =
                (rgb.0 >= 128) as u32 + 2 * (rgb.1 >= 128) as u32 + 4 * (rgb.2 >= 128) as u32;
            format!("\x1b[3{index}m")
        }
    }
}

/// One sprite row at scale 1 (32 display cells: 16 pixels of two `#`
/// glyphs each -- install.sh's own ASCII fallback fill glyph, `IUI_G_FILL`
/// in the `ascii` glyph set, used here unconditionally rather than the
/// Unicode block character its `blocks` set prefers: verified live with the
/// interactive-shell skill, `█` came out blank in that wrapper's own
/// screen model, which documents non-ASCII glyphs as approximate). Blank
/// (32 spaces) in `ColorMode::None`, mirroring install.sh's own "no colour,
/// no mascot".
pub fn head_line(mode: ColorMode, art_row: usize, eye: EyeState) -> String {
    if mode == ColorMode::None {
        return " ".repeat(32);
    }
    let row = if art_row == 8 || art_row == 9 {
        eye_row(eye)
    } else {
        ART[art_row]
    };
    let mut out = String::new();
    for hex in row.split_whitespace() {
        out.push_str(&fg_sgr(mode, hex_to_rgb(hex)));
        out.push_str("##");
    }
    out.push_str("\x1b[0m");
    out
}
pub const HEIGHT: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_art_row_has_sixteen_pixels() {
        for (i, row) in ART.iter().enumerate() {
            assert_eq!(row.split_whitespace().count(), 16, "row {i}");
        }
    }

    #[test]
    fn every_pixel_is_a_valid_six_digit_hex_code() {
        for row in ART {
            for pixel in row.split_whitespace() {
                assert_eq!(pixel.len(), 6, "pixel {pixel:?}");
                assert!(pixel.chars().all(|c| c.is_ascii_hexdigit()), "{pixel:?}");
            }
        }
    }

    #[test]
    fn hex_to_rgb_decodes_each_channel() {
        assert_eq!(hex_to_rgb("ff0080"), (255, 0, 128));
    }

    #[test]
    fn eye_states_differ_left_right_front() {
        assert_ne!(eye_row(EyeState::Left), eye_row(EyeState::Right));
        assert_ne!(eye_row(EyeState::Left), eye_row(EyeState::Front));
    }

    #[test]
    fn no_color_mode_renders_blank_and_no_escapes() {
        let line = head_line(ColorMode::None, 0, EyeState::Front);
        assert_eq!(line, " ".repeat(32));
        assert!(!line.contains('\x1b'));
    }

    #[test]
    fn true_color_mode_carries_the_exact_rgb_triple() {
        let line = head_line(ColorMode::TrueColor, 0, EyeState::Front);
        assert!(line.contains("38;2;242;207;56"));
    }

    #[test]
    fn eye_animator_dwells_on_front_before_moving() {
        let mut eyes = EyeAnimator::new();
        assert_eq!(eyes.current(), EyeState::Front);
        for _ in 0..5 {
            eyes.advance();
            assert_eq!(eyes.current(), EyeState::Front);
        }
        eyes.advance();
        assert_eq!(eyes.current(), EyeState::Right);
    }

    #[test]
    fn eye_animator_wraps_around() {
        let mut eyes = EyeAnimator::new();
        for _ in 0..EYE_FRAMES.len() {
            eyes.advance();
        }
        assert_eq!(eyes.current(), EyeState::Front);
    }
}
