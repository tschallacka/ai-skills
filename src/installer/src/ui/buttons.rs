// MODE: DEV
// PACKAGE: PROD
//! A button-coloring primitive shared by every screen in this crate's UI
//! that draws a clickable, colored control: `wizard`'s two-button screens
//! (Install now/Cancel, Use these settings/Start fresh) and the skill
//! picker's own clickable ACTIONS buttons and integration-mode toggle. The
//! rule is the same everywhere -- Tab/mouse focus is unconditional reverse
//! video (an ordinary terminal attribute, so it still shows on a
//! `ColorMode::None` terminal with no color support at all), and resting
//! color is a background SGR span gated on `ColorMode` -- so "this is the
//! interactive control" looks the same regardless of which screen drew it.

use super::mascot::{bg_sgr, fg_sgr, ColorMode};

/// Wraps `label` in a colored background SGR span (and its reset) when
/// resting, or in reverse video when `focused` -- reverse video wins
/// outright rather than combining with the background color, since
/// swapping foreground/background on top of an explicit background color
/// is exactly the kind of SGR interaction that renders differently across
/// terminals. `ColorMode::None` still gets the reverse-video focus marker
/// (an ordinary terminal attribute, not a "color"), just never the resting
/// background.
pub(crate) fn colorize_button(
    mode: ColorMode,
    label: &str,
    bg: (u8, u8, u8),
    focused: bool,
) -> String {
    if focused {
        format!("\x1b[7m{label}\x1b[0m")
    } else if mode == ColorMode::None {
        label.to_string()
    } else {
        format!("{}{label}\x1b[0m", bg_sgr(mode, bg))
    }
}

/// Colors `text`'s foreground only, with no background span and no
/// reverse-video option -- a table cell's status word ("ok"/"missing"),
/// not a clickable control, reads better as colored text than as a button.
/// A no-op on `ColorMode::None`, the same fallback every colorizer here
/// uses.
pub(crate) fn colorize_text(mode: ColorMode, text: &str, fg: (u8, u8, u8)) -> String {
    if mode == ColorMode::None {
        text.to_string()
    } else {
        format!("{}{text}\x1b[0m", fg_sgr(mode, fg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_is_always_reverse_video_regardless_of_color_mode() {
        assert_eq!(
            colorize_button(ColorMode::None, "[ OK ]", (0, 0, 0), true),
            "\x1b[7m[ OK ]\x1b[0m"
        );
        assert_eq!(
            colorize_button(ColorMode::TrueColor, "[ OK ]", (1, 2, 3), true),
            "\x1b[7m[ OK ]\x1b[0m"
        );
    }

    #[test]
    fn resting_with_no_color_support_is_plain_text() {
        assert_eq!(
            colorize_button(ColorMode::None, "[ OK ]", (1, 2, 3), false),
            "[ OK ]"
        );
    }

    #[test]
    fn resting_with_color_support_carries_the_background_span() {
        let out = colorize_button(ColorMode::TrueColor, "[ OK ]", (1, 2, 3), false);
        assert!(out.starts_with("\x1b[48;2;1;2;3m"));
        assert!(out.ends_with("[ OK ]\x1b[0m"));
    }

    #[test]
    fn colorize_text_is_plain_with_no_color_support() {
        assert_eq!(colorize_text(ColorMode::None, "ok", (1, 2, 3)), "ok");
    }

    #[test]
    fn colorize_text_carries_a_foreground_span_not_a_background_one() {
        let out = colorize_text(ColorMode::TrueColor, "ok", (1, 2, 3));
        assert!(out.starts_with("\x1b[38;2;1;2;3m"));
        assert!(!out.contains("\x1b[48"));
        assert!(out.ends_with("ok\x1b[0m"));
    }
}
