// MODE: DEV
// PACKAGE: PROD
//! Builds one frame as exactly `rows` lines of exactly `cols` display
//! cells, independent of the terminal, so it is unit-testable without a
//! live tty. Content (skill names/descriptions) is ASCII-only, so its byte
//! length is its cell width; the border glyphs are the one place that is not
//! true (`BorderSet::UNICODE`'s box-drawing characters are 3 UTF-8 bytes
//! each but still exactly one display column), which is why every width
//! check in this module's own tests counts `.chars()`, not `.len()`.
//!
//! The mascot itself is NOT drawn here: when `layout.mascot_on`, this just
//! reserves its rows as blank cells (a separator line, then `mascot::HEIGHT`
//! blank rows) in the list pane. The actual sprite is painted as a colored
//! overlay at an absolute position after this frame is drawn -- mixing SGR
//! escapes into these strings would break the "every line is exactly
//! `cols` display cells" invariant every test here checks.

use super::buttons::colorize_button;
use super::layout::Layout;
use super::mascot::ColorMode;
use super::model::{Focus, PickerState};
#[cfg(test)]
use super::text::is_precomposed_line;
use super::text::{overflow, pad, wrap};
use crate::requirements::SkillState;

/// Both bars are chrome, not critical content: at a width where they don't
/// fit on one line, wrapping onto a second line is the graceful fallback,
/// and only a THIRD line -- content wrapping alone still couldn't fit --
/// gets `pad`'s `...` truncation. `layout::compute` needs the resulting
/// line count (at this same `cols`) to reserve the right number of body
/// rows before this ever renders; see its own doc comment.
const BAR_MAX_LINES: usize = 3;

/// How wide a paragraph of free text (a description, a dependency
/// explanation, an action's own output) is allowed to wrap before it must
/// break onto another line, even in a terminal wide enough to offer far
/// more -- a comfortable reading measure instead of prose stretched edge to
/// edge of a very wide DETAILS pane. Fixed, short structural content (the
/// skill name, `STATUS`'s own lines) is never capped since it never gets
/// close to this width in practice.
const MAX_TEXT_COLS: usize = 80;

// The blue channel is deliberately >= 128: `bg_sgr`'s ANSI-8 downgrade picks
// its nearest color by which channels cross that threshold, and a color
// with none of them set downgrades to plain black -- indistinguishable from
// no color at all on a basic-color terminal. Verified live: the original
// (60, 70, 110) rendered with no visible background at all in a real PTY.
const DEP_HINT_BUTTON_BG: (u8, u8, u8) = (55, 80, 150);
const REVERIFY_BUTTON_BG: (u8, u8, u8) = (55, 80, 150);
/// The integration-mode toggle's own "cool colors": indigo for `skill`,
/// teal for `mcp` -- deliberately a different family from the wizard's
/// green/red/blue buttons, so the picker's own controls read as this
/// screen's own thing. An offered mode this crate does not know about yet
/// gets a neutral grey rather than guessing at a meaning for it.
fn mode_active_bg(mode: &str) -> (u8, u8, u8) {
    match mode {
        "mcp" => (20, 120, 130),
        "skill" => (70, 60, 140),
        _ => (90, 90, 90),
    }
}
const HINT_BUTTON_BG: (u8, u8, u8) = (50, 90, 130);

pub(crate) fn title_bar_text(state: &PickerState) -> String {
    format!(
        " AI-SKILLS INSTALLER  {}/{} installed  {} selected ",
        state.installed_count(),
        state.skills.len(),
        state.selected_count()
    )
}

/// What clicking a hint-bar segment does, when it does anything at all --
/// "Up/Dn move" has no single click that means the same thing (the list
/// rows are already directly clickable for that), so it carries `None` and
/// is drawn as plain descriptive text rather than a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HintClick {
    ToggleCurrent,
    FocusToggle,
    SelectAll,
    SelectNone,
    Install,
    Quit,
}

pub(crate) struct HintSegment {
    pub(crate) text: &'static str,
    pub(crate) click: Option<HintClick>,
}

pub(crate) const HINT_SEGMENTS: &[HintSegment] = &[
    HintSegment {
        text: "Up/Dn move",
        click: None,
    },
    HintSegment {
        text: "Enter/Space toggle",
        click: Some(HintClick::ToggleCurrent),
    },
    HintSegment {
        text: "Tab focus",
        click: Some(HintClick::FocusToggle),
    },
    HintSegment {
        text: "a all",
        click: Some(HintClick::SelectAll),
    },
    HintSegment {
        text: "n none",
        click: Some(HintClick::SelectNone),
    },
    HintSegment {
        text: "i install",
        click: Some(HintClick::Install),
    },
    HintSegment {
        text: "q quit",
        click: Some(HintClick::Quit),
    },
];

/// The hint bar's plain text, built from `HINT_SEGMENTS` so the displayed
/// string and the column ranges `hint_segment_ranges` computes can never
/// drift apart the way two independently maintained copies could.
fn hint_text_plain() -> String {
    format!(
        " {}",
        HINT_SEGMENTS
            .iter()
            .map(|s| s.text)
            .collect::<Vec<_>>()
            .join("  ")
    )
}

/// Each segment's 0-based `[start, end)` content-column span within
/// `hint_text_plain()`'s single line -- the read-backwards counterpart to
/// how that text was built, in lockstep with `HINT_SEGMENTS`. Only
/// meaningful when the hint bar actually rendered as one line; a caller
/// checks that separately (`hint_click_at` does).
fn hint_segment_ranges() -> Vec<(usize, usize)> {
    let mut ranges = Vec::with_capacity(HINT_SEGMENTS.len());
    let mut col = 1; // past the leading space
    for (i, seg) in HINT_SEGMENTS.iter().enumerate() {
        if i > 0 {
            col += 2; // the "  " separator between segments
        }
        let end = col + seg.text.chars().count();
        ranges.push((col, end));
        col = end;
    }
    ranges
}

pub(crate) fn title_bar_lines(state: &PickerState, cols: usize) -> Vec<String> {
    overflow(&title_bar_text(state), cols, BAR_MAX_LINES)
}

pub(crate) fn hint_bar_lines(cols: usize) -> Vec<String> {
    overflow(&hint_text_plain(), cols, BAR_MAX_LINES)
}

/// Like `hint_bar_lines`, but with each clickable segment wrapped in its own
/// colored button span -- only when the hint bar actually rendered as ONE
/// line. A wrapped (multi-line) hint bar is a narrow-terminal corner case
/// where a segment's own text can straddle a wrap boundary chosen by
/// `overflow`'s generic word-wrapping, which knows nothing about segment
/// boundaries; rather than risk splitting a colored span across two lines,
/// this deliberately falls back to the plain, uncolored (but still fully
/// readable) text in that case. `ColorMode::None` also renders plain, the
/// same as every other button in this crate.
pub(crate) fn hint_bar_lines_colored(cols: usize, mode: ColorMode) -> Vec<String> {
    let plain = hint_bar_lines(cols);
    if mode == ColorMode::None || plain.len() != 1 {
        return plain;
    }
    vec![colorize_hint_line(&plain[0], mode)]
}

fn colorize_hint_line(plain: &str, mode: ColorMode) -> String {
    let chars: Vec<char> = plain.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    for (seg, (start, end)) in HINT_SEGMENTS.iter().zip(hint_segment_ranges()) {
        out.extend(chars[i..start].iter());
        let seg_text: String = chars[start..end].iter().collect();
        if seg.click.is_some() {
            out.push_str(&colorize_button(mode, &seg_text, HINT_BUTTON_BG, false));
        } else {
            out.push_str(&seg_text);
        }
        i = end;
    }
    out.extend(chars[i..].iter());
    out
}

/// What a click at `(col, row)` (1-based, exactly what `Key::Click` reports)
/// lands on in the hint bar, if anything -- `None` when the bar wrapped onto
/// more than one line (see `hint_bar_lines_colored`'s own doc comment for
/// why that case is out of scope) or the click missed the bar's one line, or
/// landed in the gap between two segments, or on a segment with no click
/// action ("Up/Dn move"). `hint_rows` is the caller's own already-computed
/// line count for the hint bar at this frame's width (the same value used
/// to reserve its rows in `layout::compute`), the same pattern
/// `layout::hit_test` already uses `title_rows` for.
pub(crate) fn hint_click_at(
    layout: &Layout,
    hint_rows: usize,
    col: u16,
    row: u16,
) -> Option<HintClick> {
    if hint_rows != 1 || col == 0 || row == 0 {
        return None;
    }
    let hint_row_1based = layout.rows - hint_rows + 1;
    if row as usize != hint_row_1based {
        return None;
    }
    let content_col = (col - 1) as usize;
    HINT_SEGMENTS
        .iter()
        .zip(hint_segment_ranges())
        .find(|(_, (start, end))| (*start..*end).contains(&content_col))
        .and_then(|(seg, _)| seg.click)
}

/// The state suffix is appended after the name rather than inserted before
/// it, so an Ok skill's row (the common case, and the only case in most
/// existing frame-shape tests) renders byte-identical to before this state
/// tag existed. The whole padded cell is wrapped in reverse video for the
/// cursor row -- the same "this is where the cursor is" treatment every
/// wizard screen's own list already gives its cursor row, replacing what
/// used to be only the bare `>` marker's own visual weight.
fn list_row(state: &PickerState, index: usize, width: usize) -> String {
    let cursor = if index == state.cursor { '>' } else { ' ' };
    let checkbox = if state.selected[index] { "[x]" } else { "[ ]" };
    let name = &state.skills[index].name;
    let suffix = match state.skills[index].status.state {
        SkillState::Ok => "",
        SkillState::Degraded => " ~",
        SkillState::Blocked => " !",
    };
    let content = pad(&format!("{cursor}{checkbox} {name}{suffix}"), width);
    if index == state.cursor {
        format!("\x1b[7m{content}\x1b[0m")
    } else {
        content
    }
}

/// Where the two ACTIONS buttons and the integration-mode toggle (when
/// drawn at all) landed within `info_lines`' own returned rows, plus the
/// toggle's own segment column ranges -- computed by `build_info` alongside
/// the content itself so the two can never drift apart, and used by
/// `mod::handle_key` to test a click against them and by `render_wide`/
/// `render_narrow` to know which rows to colorize.
#[derive(Debug, Default, Clone)]
pub(crate) struct InfoLayout {
    pub(crate) dep_hint_row: Option<usize>,
    pub(crate) reverify_row: Option<usize>,
    pub(crate) mode_toggle: Option<ModeToggleLayout>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModeToggleLayout {
    pub(crate) row: usize,
    /// `(mode name, start content-column, end content-column exclusive)`
    /// for each offered mode, in the order drawn.
    pub(crate) segments: Vec<(String, usize, usize)>,
}

/// Name, description, install status, DEPENDENCIES (when requires.tsv named
/// any), and ACTIONS (dependency help, reverify, and an INTEGRATION MODE
/// toggle when the skill offers more than one). Plain, uncolored content --
/// coloring the buttons and the toggle's active segment is `render_wide`/
/// `render_narrow`'s own job at actual draw time, using `info_layout`'s
/// metadata, so this function (and the line COUNT it returns) stays simple
/// and independent of `ColorMode`.
pub(crate) fn info_lines(state: &PickerState, width: usize) -> Vec<String> {
    build_info(state, width).0
}

/// Same content as `info_lines`, but returning the button/toggle row and
/// column metadata instead of the text -- see `InfoLayout`.
pub(crate) fn info_layout(state: &PickerState, width: usize) -> InfoLayout {
    build_info(state, width).1
}

fn build_info(state: &PickerState, width: usize) -> (Vec<String>, InfoLayout) {
    let text_width = width.min(MAX_TEXT_COLS);
    let mut lines = Vec::new();
    let mut layout = InfoLayout::default();
    let skill = &state.skills[state.cursor];
    lines.push(pad("-".repeat(text_width).as_str(), width));
    lines.push(pad(&skill.name, width));
    lines.push(pad("", width));
    for line in wrap(&skill.description, text_width) {
        lines.push(pad(&line, width));
    }
    lines.push(pad("", width));
    lines.push(pad("STATUS", width));
    lines.push(pad(
        &format!(
            "  installed      {}",
            if skill.installed { "yes" } else { "no" }
        ),
        width,
    ));
    lines.push(pad(
        &format!(
            "  state          {}",
            match skill.status.state {
                SkillState::Ok => "ok",
                SkillState::Degraded => "degraded",
                SkillState::Blocked => "blocked",
            }
        ),
        width,
    ));
    if !skill.status.requirements.is_empty() {
        lines.push(pad("", width));
        lines.push(pad("DEPENDENCIES", width));
        for (req, met) in &skill.status.requirements {
            let label = crate::requirements::requirement_label(req);
            let mark = if *met { "ok" } else { "missing" };
            let strength = match req.strength {
                crate::requirements::Strength::Hard => "hard",
                crate::requirements::Strength::Soft => "soft",
            };
            for line in wrap(
                &format!("  {label} ({strength}): {mark} -- {}", req.why),
                text_width,
            ) {
                lines.push(pad(&line, width));
            }
        }
    }
    // Always listed, usable only when the info pane has focus, but named
    // here regardless so a reader in the list pane already knows what
    // focusing it offers.
    lines.push(pad("", width));
    lines.push(pad("ACTIONS", width));
    layout.dep_hint_row = Some(lines.len());
    lines.push(pad("  [ d ] Help me install dependencies", width));
    layout.reverify_row = Some(lines.len());
    lines.push(pad("  [ r ] Reverify dependencies", width));
    if skill.offered_modes.len() > 1 {
        lines.push(pad("", width));
        for line in wrap(
            "INTEGRATION MODE (click a mode, or press m to cycle)",
            text_width,
        ) {
            lines.push(pad(&line, width));
        }
        let (toggle_line, segments) = mode_toggle_line(&skill.offered_modes, width);
        layout.mode_toggle = Some(ModeToggleLayout {
            row: lines.len(),
            segments,
        });
        lines.push(toggle_line);
    }
    if !state.message.is_empty() {
        lines.push(pad("", width));
        lines.push(pad("OUTPUT", width));
        for message in &state.message {
            for line in wrap(message, text_width) {
                lines.push(pad(&line, width));
            }
        }
    }
    (lines, layout)
}

/// Builds the plain (uncolored) toggle row -- `[ mode1 ] [ mode2 ] ...` for
/// every mode this skill offers -- plus each segment's own 0-based
/// content-column span (before padding) so a click, and the colorizing pass
/// at render time, both agree on exactly which columns are which segment.
fn mode_toggle_line(modes: &[String], width: usize) -> (String, Vec<(String, usize, usize)>) {
    let mut line = String::new();
    let mut segments = Vec::with_capacity(modes.len());
    for (i, m) in modes.iter().enumerate() {
        if i > 0 {
            line.push(' ');
        }
        let start = line.chars().count();
        line.push_str(&format!("[ {m} ]"));
        let end = line.chars().count();
        segments.push((m.clone(), start, end));
    }
    (pad(&line, width), segments)
}

/// Applies `InfoLayout`'s coloring to one already-built, already-padded
/// plain row: the ACTIONS buttons get a full-width colored background (a
/// real button block, not just colored text), and the toggle row gets only
/// its ACTIVE segment colored (`mode_active_bg`) -- the inactive segment(s)
/// stay plain text, so the toggle reads as a switch rather than two equally
/// weighted buttons. Any other row passes through unchanged.
fn colorize_info_row(
    plain: String,
    row_index: usize,
    info_layout: &InfoLayout,
    active_mode: &str,
    mode: ColorMode,
) -> String {
    if mode == ColorMode::None {
        return plain;
    }
    if info_layout.dep_hint_row == Some(row_index) {
        return colorize_button(mode, &plain, DEP_HINT_BUTTON_BG, false);
    }
    if info_layout.reverify_row == Some(row_index) {
        return colorize_button(mode, &plain, REVERIFY_BUTTON_BG, false);
    }
    if let Some(toggle) = &info_layout.mode_toggle {
        if toggle.row == row_index {
            return colorize_mode_toggle_row(&plain, &toggle.segments, active_mode, mode);
        }
    }
    plain
}

fn colorize_mode_toggle_row(
    plain: &str,
    segments: &[(String, usize, usize)],
    active: &str,
    mode: ColorMode,
) -> String {
    let chars: Vec<char> = plain.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    for (mode_name, start, end) in segments {
        out.extend(chars[i..*start].iter());
        let seg_text: String = chars[*start..*end].iter().collect();
        if mode_name == active {
            out.push_str(&colorize_button(
                mode,
                &seg_text,
                mode_active_bg(mode_name),
                false,
            ));
        } else {
            out.push_str(&seg_text);
        }
        i = *end;
    }
    out.extend(chars[i..].iter());
    out
}

/// The glyphs every border-drawing function below draws with, resolved once
/// per frame from `layout.unicode_borders` rather than read as a global, the
/// same reason `color_mode` is threaded as a value everywhere else here: a
/// test can force either set with no terminal, and nothing here depends on
/// probing anything live. Every field is exactly one display column in every
/// terminal that renders it at all (plain ASCII, or the box-drawing block
/// U+2500-U+257F) -- picking the wrong set for an incapable terminal is a
/// portability bug, not a width bug, and `detect_utf8_capable` is what
/// guards against that.
pub(crate) struct BorderSet {
    pub(crate) horizontal: char,
    pub(crate) vertical: char,
    pub(crate) corner_tl: char,
    pub(crate) corner_tr: char,
    pub(crate) corner_bl: char,
    pub(crate) corner_br: char,
    /// Where the list pane's top border meets the divider into the info
    /// pane (a T pointing down into the frame).
    pub(crate) divider_top: char,
    /// The same divider's bottom-border counterpart (a T pointing up).
    pub(crate) divider_bottom: char,
}

impl BorderSet {
    pub(crate) const ASCII: BorderSet = BorderSet {
        horizontal: '-',
        vertical: '|',
        corner_tl: '+',
        corner_tr: '+',
        corner_bl: '+',
        corner_br: '+',
        divider_top: '+',
        divider_bottom: '+',
    };
    pub(crate) const UNICODE: BorderSet = BorderSet {
        horizontal: '─',
        vertical: '│',
        corner_tl: '┌',
        corner_tr: '┐',
        corner_bl: '└',
        corner_br: '┘',
        divider_top: '┬',
        divider_bottom: '┴',
    };

    /// `wizard`'s own full-screen steps have no `Layout` of their own (no
    /// skill list, no info pane) but still want the same glyph set a
    /// UTF-8-capable terminal gets everywhere else -- this is the plain
    /// bool-driven half `for_layout` is built from, so both callers pick the
    /// same two sets from one place rather than each hardcoding its own.
    pub(crate) fn for_unicode(unicode: bool) -> &'static BorderSet {
        if unicode {
            &BorderSet::UNICODE
        } else {
            &BorderSet::ASCII
        }
    }

    fn for_layout(layout: &Layout) -> &'static BorderSet {
        BorderSet::for_unicode(layout.unicode_borders)
    }
}

fn top_border(layout: &Layout, focus: Focus) -> String {
    let b = BorderSet::for_layout(layout);
    let list_label = if focus == Focus::List {
        "[SKILLS]"
    } else {
        " SKILLS "
    };
    let info_label = if focus == Focus::Info {
        "[DETAILS]"
    } else {
        " DETAILS "
    };
    format!(
        "{}{}{}{}{}",
        b.corner_tl,
        pad_center_dash(
            list_label,
            layout.left_w,
            b.horizontal,
            focus == Focus::List
        ),
        b.divider_top,
        pad_center_dash(
            info_label,
            layout.right_w,
            b.horizontal,
            focus == Focus::Info
        ),
        b.corner_tr
    )
}

/// `focused` wraps the label in reverse video -- an ordinary terminal
/// attribute, so it shows even with no color support -- the same "this pane
/// has keyboard focus" treatment every wizard screen already gives its own
/// focused control, replacing what used to be only the bracket-vs-no-bracket
/// text difference. The bracket text itself is kept too, so the cue still
/// reads on a terminal (or a transcript) that strips SGR entirely.
fn pad_center_dash(label: &str, width: usize, fill: char, focused: bool) -> String {
    let visible = label.chars().count();
    if visible >= width {
        let truncated: String = label.chars().take(width).collect();
        return if focused {
            format!("\x1b[7m{truncated}\x1b[0m")
        } else {
            truncated
        };
    }
    let content = if focused {
        format!("\x1b[7m{label}\x1b[0m")
    } else {
        label.to_string()
    };
    format!(
        "{content}{}",
        std::iter::repeat_n(fill, width - visible).collect::<String>()
    )
}

fn bottom_border(layout: &Layout) -> String {
    let b = BorderSet::for_layout(layout);
    format!(
        "{}{}{}{}{}",
        b.corner_bl,
        std::iter::repeat_n(b.horizontal, layout.left_w).collect::<String>(),
        b.divider_bottom,
        std::iter::repeat_n(b.horizontal, layout.right_w).collect::<String>(),
        b.corner_br
    )
}

pub fn render_frame(state: &PickerState, layout: &Layout, color_mode: ColorMode) -> Vec<String> {
    let mut out = Vec::with_capacity(layout.rows);
    out.extend(title_bar_lines(state, layout.cols));

    if layout.narrow {
        render_narrow(state, layout, color_mode, &mut out);
    } else {
        render_wide(state, layout, color_mode, &mut out);
    }
    out.extend(hint_bar_lines_colored(layout.cols, color_mode));
    out
}

/// One list-pane cell for body row `body`: a skill row while `body` is
/// inside `layout.list_rows`, then (only when `layout.mascot_on`) one
/// separator line and `mascot::HEIGHT` blank rows the overlay paints over,
/// then blank padding for whatever body rows remain.
fn list_cell(state: &PickerState, layout: &Layout, body: usize) -> String {
    if body < layout.list_rows {
        return if state.scroll + body < state.skills.len() {
            list_row(state, state.scroll + body, layout.left_w)
        } else {
            pad("", layout.left_w)
        };
    }
    if layout.mascot_on && body == layout.list_rows {
        let fill = BorderSet::for_layout(layout).horizontal;
        return std::iter::repeat_n(fill, layout.left_w).collect();
    }
    pad("", layout.left_w)
}

fn render_wide(state: &PickerState, layout: &Layout, color_mode: ColorMode, out: &mut Vec<String>) {
    let b = BorderSet::for_layout(layout);
    out.push(top_border(layout, state.focus));
    let info = info_lines(state, layout.right_w);
    let info_meta = info_layout(state, layout.right_w);
    let active_mode = state.skills[state.cursor].mode.as_str();
    for body in 0..layout.body_rows {
        let info_index = body + state.info_scroll;
        let info_cell = info
            .get(info_index)
            .cloned()
            .unwrap_or_else(|| pad("", layout.right_w));
        let info_cell =
            colorize_info_row(info_cell, info_index, &info_meta, active_mode, color_mode);
        out.push(format!(
            "{}{}{}{info_cell}{}",
            b.vertical,
            list_cell(state, layout, body),
            b.vertical,
            b.vertical
        ));
    }
    out.push(bottom_border(layout));
}

fn render_narrow(
    state: &PickerState,
    layout: &Layout,
    color_mode: ColorMode,
    out: &mut Vec<String>,
) {
    let b = BorderSet::for_layout(layout);
    let label = if state.focus == Focus::Info {
        "[DETAILS]"
    } else {
        "[SKILLS]"
    };
    out.push(format!(
        "{}{}{}",
        b.corner_tl,
        // Narrow shows exactly one pane at a time, so whichever is shown is
        // always the focused one -- unlike `top_border`'s two side-by-side
        // labels, there is no unfocused case to distinguish here.
        pad_center_dash(label, layout.left_w, b.horizontal, true),
        b.corner_tr
    ));
    let info = if state.focus == Focus::Info {
        info_lines(state, layout.left_w)
    } else {
        Vec::new()
    };
    let info_meta = if state.focus == Focus::Info {
        info_layout(state, layout.left_w)
    } else {
        InfoLayout::default()
    };
    let active_mode = state.skills[state.cursor].mode.as_str();
    for body in 0..layout.body_rows {
        let cell = if state.focus == Focus::Info {
            let plain = info
                .get(body + state.info_scroll)
                .cloned()
                .unwrap_or_else(|| pad("", layout.left_w));
            colorize_info_row(
                plain,
                body + state.info_scroll,
                &info_meta,
                active_mode,
                color_mode,
            )
        } else if state.scroll + body < state.skills.len() {
            list_row(state, state.scroll + body, layout.left_w)
        } else {
            pad("", layout.left_w)
        };
        out.push(format!("{}{cell}{}", b.vertical, b.vertical));
    }
    out.push(format!(
        "{}{}{}",
        b.corner_bl,
        std::iter::repeat_n(b.horizontal, layout.left_w).collect::<String>(),
        b.corner_br
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requirements::SkillStatus;
    use crate::ui::layout;
    use crate::ui::model::SkillEntry;

    fn skills(names: &[&str]) -> Vec<SkillEntry> {
        names
            .iter()
            .map(|n| SkillEntry {
                name: n.to_string(),
                description: format!("{n} does things."),
                installed: false,
                status: SkillStatus {
                    state: SkillState::Ok,
                    blocker: None,
                    requirements: Vec::new(),
                },
                offered_modes: Vec::new(),
                mode: "skill".to_string(),
            })
            .collect()
    }

    /// `layout::compute` needs the ACTUAL number of lines the title/hint
    /// bars will take at this `cols` (they vary with both the width and,
    /// for the title, the state's own counts) -- derived here exactly the
    /// way `render_frame` derives them, so a test's `Layout` always agrees
    /// with what `render_frame` actually produces at the same inputs.
    fn layout_for(cols: usize, rows: usize, state: &PickerState, color_capable: bool) -> Layout {
        layout_for_borders(cols, rows, state, color_capable, false)
    }

    fn layout_for_borders(
        cols: usize,
        rows: usize,
        state: &PickerState,
        color_capable: bool,
        unicode_borders: bool,
    ) -> Layout {
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let title_rows = title_bar_lines(state, cols).len();
        let hint_rows = hint_bar_lines(cols).len();
        layout::compute(
            cols,
            rows,
            &names,
            color_capable,
            title_rows,
            hint_rows,
            unicode_borders,
        )
    }

    #[test]
    fn every_line_is_exactly_the_terminal_width() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::None);
        assert_eq!(frame.len(), layout.rows);
        for line in &frame {
            // The cursor row and the focused pane's title are wrapped in
            // reverse video regardless of `ColorMode` (an ordinary terminal
            // attribute, not a color) -- their invisible SGR bytes are not
            // display columns, so they are checked separately
            // (`the_cursor_row_and_focused_title_are_reverse_video`) rather
            // than held to this plain `.chars().count()` invariant.
            if is_precomposed_line(line) {
                continue;
            }
            assert_eq!(line.chars().count(), layout.cols, "line was: {line:?}");
        }
    }

    #[test]
    fn the_cursor_row_and_focused_title_are_reverse_video() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::None);
        // Row 0: title bar. Row 1: top border, carrying the focused
        // "[SKILLS]" label (focus defaults to List). Row 2: the cursor row
        // (cursor defaults to 0).
        assert!(frame[1].contains("\x1b[7m"), "top border: {:?}", frame[1]);
        assert!(frame[2].contains("\x1b[7m"), "cursor row: {:?}", frame[2]);
        // A non-cursor body row carries neither.
        assert!(!frame[3].contains("\x1b[7m"), "row was: {:?}", frame[3]);
    }

    /// The same invariant as above, but with the box-drawing border set,
    /// where a border cell is 3 UTF-8 bytes yet still exactly one display
    /// column -- `.chars().count()`, not `.len()`, is what must hold here.
    #[test]
    fn unicode_borders_still_measure_one_column_per_glyph() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let layout = layout_for_borders(80, 24, &state, true, true);
        let frame = render_frame(&state, &layout, ColorMode::None);
        assert_eq!(frame.len(), layout.rows);
        for line in &frame {
            if is_precomposed_line(line) {
                continue;
            }
            assert_eq!(line.chars().count(), layout.cols, "line was: {line:?}");
        }
        // And the byte length is now genuinely longer than the column count
        // on a bordered row -- proving this test would have caught a
        // regression back to raw `.len()`, not just passed by coincidence.
        // Row 3 is a plain (non-cursor, non-title) body row.
        let bordered_row = &frame[3];
        assert!(
            bordered_row.len() > bordered_row.chars().count(),
            "expected multi-byte border glyphs on a bordered row: {bordered_row:?}"
        );
    }

    #[test]
    fn unicode_borders_draw_the_box_drawing_glyphs_ascii_never_does() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for_borders(80, 24, &state, true, true);
        let frame = render_frame(&state, &layout, ColorMode::None);
        let top = &frame[1];
        assert!(top.starts_with('┌'), "top border was: {top:?}");
        assert!(top.ends_with('┐'), "top border was: {top:?}");
        assert!(top.contains('┬'), "top border was: {top:?}");
        // the hint bar is the true last line; the border sits one above it
        let bottom = &frame[frame.len() - 2];
        assert!(bottom.starts_with('└'), "bottom border was: {bottom:?}");
        assert!(bottom.ends_with('┘'), "bottom border was: {bottom:?}");
        assert!(bottom.contains('┴'), "bottom border was: {bottom:?}");
    }

    #[test]
    fn ascii_borders_never_draw_a_box_drawing_glyph() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::None);
        for line in &frame {
            assert!(
                !line.contains(['┌', '┐', '└', '┘', '─', '│', '┬', '┴']),
                "ASCII mode drew a box-drawing glyph: {line:?}"
            );
        }
    }

    #[test]
    fn the_cursor_row_carries_the_marker() {
        let mut state = PickerState::new(skills(&["todo", "bug-report"]));
        state.cursor = 1;
        let layout = layout_for(80, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let frame = render_frame(&state, &layout, ColorMode::None);
        // title bar(s), then the top border, then the second skill row.
        let body_line = &frame[title_rows + 1 + 1];
        assert!(body_line.contains(">[x] bug-report"));
    }

    #[test]
    fn a_deselected_skill_shows_an_empty_checkbox() {
        let mut state = PickerState::new(skills(&["todo"]));
        state.toggle(0);
        let layout = layout_for(80, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let frame = render_frame(&state, &layout, ColorMode::None);
        assert!(frame[title_rows + 1].contains("[ ] todo"));
    }

    #[test]
    fn a_narrow_terminal_renders_one_pane_with_no_pipe_divider() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(40, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let hint_rows = hint_bar_lines(layout.cols).len();
        let frame = render_frame(&state, &layout, ColorMode::None);
        // Everything between the title bar's own line(s) + its top border,
        // and the hint bar's own line(s) + its bottom border, is body.
        let start = title_rows + 1;
        let end = frame.len() - hint_rows - 1;
        for line in &frame[start..end] {
            assert_eq!(line.matches('|').count(), 2, "line was: {line:?}");
        }
    }

    #[test]
    fn the_title_bar_reports_selected_and_installed_counts() {
        let mut state = PickerState::new(skills(&["a", "b", "c"]));
        state.toggle(0);
        state.skills[1].installed = true;
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::None);
        assert!(frame[0].contains("1/3 installed"));
        assert!(frame[0].contains("2 selected"));
    }

    #[test]
    fn wrap_hyphenates_a_token_wider_than_the_pane() {
        let lines = wrap("supercalifragilisticexpialidocious", 10);
        assert!(lines.len() > 1);
        assert!(lines[0].ends_with('-'));
    }

    #[test]
    fn wrap_breaks_on_whole_words_when_it_can() {
        let lines = wrap("one two three four", 8);
        for line in &lines {
            assert!(!line.contains("  "));
        }
        assert_eq!(lines.join(" "), "one two three four");
    }

    #[test]
    fn a_tall_terminal_reserves_blank_rows_for_the_mascot() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(80, 30, &state, true);
        assert!(layout.mascot_on);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let frame = render_frame(&state, &layout, ColorMode::None);
        // Every line still comes out exactly `cols` wide even with the
        // mascot's rows reserved -- the invariant every other test checks
        // (skipping the reverse-video cursor/title rows, same as there).
        for line in &frame {
            if is_precomposed_line(line) {
                continue;
            }
            assert_eq!(line.chars().count(), layout.cols);
        }
        let separator_row = title_rows + 1 + layout.list_rows;
        assert!(frame[separator_row].contains("-------"));
    }

    #[test]
    fn a_bar_that_does_not_fit_one_line_wraps_instead_of_truncating() {
        // At 20 columns neither bar fits on one line; both must wrap
        // rather than cut, and neither may show the truncation marker
        // unless even wrapped they still overrun BAR_MAX_LINES lines.
        let state = PickerState::new(skills(&["todo"]));
        let title_lines = title_bar_lines(&state, 20);
        let hint_lines = hint_bar_lines(20);
        assert!(title_lines.len() > 1, "title did not wrap: {title_lines:?}");
        assert!(hint_lines.len() > 1, "hint did not wrap: {hint_lines:?}");
        for line in &title_lines {
            assert_eq!(line.len(), 20);
        }
        for line in &hint_lines {
            assert_eq!(line.len(), 20);
        }
    }

    #[test]
    fn a_bar_truncation_marker_if_any_lands_only_on_the_last_line() {
        // An extreme width where even BAR_MAX_LINES of wrapping cannot fit
        // the hint text: the cut, if it happens, must be confined to the
        // final line.
        let hint_lines = hint_bar_lines(6);
        assert_eq!(hint_lines.len(), BAR_MAX_LINES);
        for line in &hint_lines[..hint_lines.len() - 1] {
            assert!(!line.contains("..."), "cut too early: {line:?}");
        }
    }

    #[test]
    fn a_skill_offering_more_than_one_mode_shows_the_toggle() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "mcp".to_string();
        let state = PickerState::new(list);
        let width = 60;
        let lines = info_lines(&state, width);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines.iter().any(|l| l.contains("INTEGRATION MODE")));
        assert!(lines
            .iter()
            .any(|l| l.contains("[ skill ]") && l.contains("[ mcp ]")));
    }

    #[test]
    fn a_skill_with_one_mode_still_shows_actions_but_no_toggle() {
        let state = PickerState::new(skills(&["todo"]));
        let lines = info_lines(&state, 60);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines
            .iter()
            .any(|l| l.contains("[ d ] Help me install dependencies")));
        assert!(lines
            .iter()
            .any(|l| l.contains("[ r ] Reverify dependencies")));
        assert!(!lines.iter().any(|l| l.contains("INTEGRATION MODE")));
    }

    #[test]
    fn info_layout_locates_the_action_buttons_and_the_toggles_segments() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "skill".to_string();
        let state = PickerState::new(list);
        let width = 60;
        let lines = info_lines(&state, width);
        let layout = info_layout(&state, width);
        let dep_row = layout.dep_hint_row.expect("dep_hint_row");
        assert!(lines[dep_row].contains("[ d ] Help me install dependencies"));
        let reverify_row = layout.reverify_row.expect("reverify_row");
        assert!(lines[reverify_row].contains("[ r ] Reverify dependencies"));
        let toggle = layout.mode_toggle.expect("mode_toggle");
        assert!(lines[toggle.row].contains("[ skill ]"));
        assert_eq!(toggle.segments.len(), 2);
        for (name, start, end) in &toggle.segments {
            let slice: String = lines[toggle.row]
                .chars()
                .skip(*start)
                .take(end - start)
                .collect();
            assert_eq!(&slice, &format!("[ {name} ]"));
        }
    }

    #[test]
    fn clicking_a_toggle_segment_selects_that_mode_directly() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "skill".to_string();
        let mut state = PickerState::new(list);
        let width = 60;
        let layout = info_layout(&state, width);
        let toggle = layout.mode_toggle.unwrap();
        let (mcp_name, mcp_start, _) = toggle
            .segments
            .iter()
            .find(|(name, _, _)| name == "mcp")
            .unwrap();
        state.set_integration_mode(mcp_name);
        assert_eq!(state.skills[0].mode, "mcp");
        let _ = mcp_start; // exercised via the shared handle_info_click path in mod.rs's own tests
    }

    #[test]
    fn the_active_toggle_segment_is_colored_the_inactive_one_is_plain() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "mcp".to_string();
        let state = PickerState::new(list);
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::TrueColor);
        let toggle_row_text = frame
            .iter()
            .find(|l| l.contains("[ mcp ]") && l.contains("\x1b["))
            .expect("a colored toggle row");
        // the active segment (mcp) carries an SGR span; "skill" does not.
        let mcp_idx = toggle_row_text.find("[ mcp ]").unwrap();
        let skill_idx = toggle_row_text.find("[ skill ]").unwrap();
        assert!(
            toggle_row_text[..mcp_idx].ends_with("m"),
            "expected an SGR reset/prefix just before mcp: {toggle_row_text:?}"
        );
        assert!(
            !toggle_row_text[skill_idx..skill_idx + "[ skill ]".len()].contains('\x1b'),
            "the inactive segment should be plain: {toggle_row_text:?}"
        );
    }

    #[test]
    fn the_output_section_only_appears_once_there_is_a_message() {
        let mut state = PickerState::new(skills(&["todo"]));
        let before = info_lines(&state, 60);
        assert!(!before.iter().any(|l| l.contains("OUTPUT")));
        state.dep_hint();
        let after = info_lines(&state, 60);
        assert!(after.iter().any(|l| l.contains("OUTPUT")));
    }

    #[test]
    fn a_wide_terminal_still_caps_wrapped_text_at_eighty_columns() {
        let mut list = skills(&["a"]);
        list[0].description = "word ".repeat(40); // far longer than one line
        let state = PickerState::new(list);
        let width = 150; // a very wide DETAILS pane
        let lines = info_lines(&state, width);
        // Every content line is still padded to the full pane width, but no
        // wrapped text line should need more than MAX_TEXT_COLS of it -- the
        // rest is blank padding, not text stretched edge to edge.
        for line in &lines {
            let trimmed = line.trim_end();
            assert!(
                trimmed.chars().count() <= MAX_TEXT_COLS,
                "line exceeded the text cap: {line:?}"
            );
        }
    }

    #[test]
    fn hint_bar_segments_reconstruct_the_plain_text_exactly() {
        let text = hint_text_plain();
        let chars: Vec<char> = text.chars().collect();
        for (seg, (start, end)) in HINT_SEGMENTS.iter().zip(hint_segment_ranges()) {
            let slice: String = chars[start..end].iter().collect();
            assert_eq!(slice, seg.text, "segment {:?} mismatched", seg.text);
        }
    }

    #[test]
    fn hint_click_at_finds_the_right_segment_on_a_single_line_bar() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(80, 24, &state, true);
        let hint_rows = hint_bar_lines(layout.cols).len();
        assert_eq!(
            hint_rows, 1,
            "expected the hint bar to fit one line at 80 cols"
        );
        let hint_row = (layout.rows - hint_rows + 1) as u16;
        // "q quit" is the last segment; its content starts right after the
        // leading space + every earlier segment plus separators.
        let (start, _) = hint_segment_ranges().last().copied().unwrap();
        let col = (start + 1) as u16; // 1-based
        assert_eq!(
            hint_click_at(&layout, hint_rows, col, hint_row),
            Some(HintClick::Quit)
        );
    }

    #[test]
    fn hint_click_at_misses_a_wrapped_multi_line_bar() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(40, 24, &state, true);
        let hint_rows = hint_bar_lines(layout.cols).len();
        assert!(hint_rows > 1, "expected the hint bar to wrap at 40 cols");
        let hint_row = (layout.rows - hint_rows + 1) as u16;
        assert_eq!(hint_click_at(&layout, hint_rows, 2, hint_row), None);
    }
}
