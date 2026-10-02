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

use super::buttons::{colorize_button, colorize_text};
use super::layout::Layout;
use super::mascot::ColorMode;
use super::model::{Focus, PickerState};
#[cfg(test)]
use super::text::is_precomposed_line;
use super::text::{overflow, pad, pad_display, wrap};
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
// A different hue from the dep-hint button, deliberately -- two adjacent
// buttons in the same color read as one compound control rather than two
// distinct actions.
const CHECK_AGAIN_BUTTON_BG: (u8, u8, u8) = (40, 115, 120);
const DEPENDENCY_OK_FG: (u8, u8, u8) = (60, 180, 90);
const DEPENDENCY_MISSING_FG: (u8, u8, u8) = (210, 80, 80);
// Three distinct hues for the three PLUGIN BUTTONS -- green for "install
// just this one", amber for the bulk "install everything selected", red for
// "quit" -- so the three read as three different kinds of action (scope,
// bulk, exit) rather than a row of interchangeable buttons. Each has at
// least one channel >= 128, the same `bg_sgr` ANSI-8 downgrade constraint
// every other button color here already follows (see `DEP_HINT_BUTTON_BG`'s
// own comment).
const INSTALL_THIS_BUTTON_BG: (u8, u8, u8) = (50, 140, 70);
const INSTALL_ALL_BUTTON_BG: (u8, u8, u8) = (170, 130, 40);
const QUIT_BUTTON_BG: (u8, u8, u8) = (150, 50, 50);
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

/// The STATUS section's own "version" line, in the user's own words rather
/// than the enum's variant names -- `None` for `NotInstalled`, since that
/// case is `build_info`'s own cue to print no version line at all (see its
/// call site).
fn version_status_word(status: crate::cli_mode::VersionStatus) -> Option<&'static str> {
    use crate::cli_mode::VersionStatus;
    match status {
        VersionStatus::NotInstalled => None,
        VersionStatus::UpToDate => Some("up to date"),
        VersionStatus::WouldUpdate => Some("would update"),
        VersionStatus::Unknown => Some("unknown (installed before version tracking)"),
    }
}

/// One `SkillRootStatus`'s own summary word, for the per-root STATUS
/// breakdown -- unlike `version_status_word`, this always returns
/// something: a root a skill is not installed on says so plainly rather
/// than being left out of the list (silently dropping it would read as "no
/// answer for this root" rather than "not installed here").
fn root_status_word(installed: bool, version_status: crate::cli_mode::VersionStatus) -> String {
    if !installed {
        return "not installed".to_string();
    }
    match version_status_word(version_status) {
        Some(word) => format!("installed, {word}"),
        None => "installed".to_string(),
    }
}

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

/// The 0-based content-column span of a list row's own `[x]`/`[ ]` checkbox
/// -- column 0 is the cursor marker (`>`/` `), 1..4 the checkbox glyph
/// itself, the rest the skill's name. `mod::handle_key` uses this to toggle
/// selection only when a click actually lands on the checkbox, rather than
/// anywhere in the row: a click elsewhere still moves the cursor and focus
/// there, but leaves what is checked alone, the same way clicking a skill's
/// name to read about it in DETAILS should not also silently deselect it.
pub(crate) const LIST_ROW_CHECKBOX_COLS: std::ops::Range<usize> = 1..4;

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

/// Where the two ACTIONS buttons, the dependency table's own status cells,
/// and the integration-mode toggle (when drawn at all) landed within
/// `info_lines`' own returned rows -- computed by `build_info` alongside the
/// content itself so the two can never drift apart, and used by
/// `mod::handle_key` to test a click against them and by `render_wide`/
/// `render_narrow` to know which rows to colorize.
#[derive(Debug, Default, Clone)]
pub(crate) struct InfoLayout {
    pub(crate) actions: Option<ActionButtonsLayout>,
    pub(crate) mode_toggle: Option<ModeToggleLayout>,
    pub(crate) status_cells: Vec<StatusCell>,
    pub(crate) plugin_buttons: Option<PluginButtonsLayout>,
}

/// The two side-by-side ACTIONS buttons' own column ranges within their
/// shared row -- unlike the mode toggle's N segments, this is always
/// exactly two, named rather than indexed, since the two actions are
/// different verbs, not interchangeable options.
#[derive(Debug, Clone)]
pub(crate) struct ActionButtonsLayout {
    pub(crate) row: usize,
    pub(crate) dep_hint: (usize, usize),
    pub(crate) check_again: (usize, usize),
}

/// One dependency table row's own "ok"/"missing" cell -- colored green or
/// red at render time, the same deferred-coloring pattern the action
/// buttons and mode toggle already use, so `info_lines`'s plain content
/// stays independent of `ColorMode`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StatusCell {
    pub(crate) row: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) ok: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ModeToggleLayout {
    pub(crate) row: usize,
    /// `(mode name, start content-column, end content-column exclusive)`
    /// for each offered mode, in the order drawn.
    pub(crate) segments: Vec<(String, usize, usize)>,
}

/// The DETAILS pane's own global controls, present for every skill
/// regardless of its own dependency state -- unlike `ActionButtonsLayout`
/// (only there when something is missing) or `ModeToggleLayout` (only there
/// for a multi-mode skill), these three buttons always draw: they are the
/// picker's install controls, placed where a mouse-first user is already
/// looking. "Install/update this skill"
/// gets its own row; the bulk "install/update all" and "quit" share the row
/// below it, the same side-by-side-buttons shape `ActionButtonsLayout`
/// already uses for its own pair.
#[derive(Debug, Clone)]
pub(crate) struct PluginButtonsLayout {
    pub(crate) install_this_row: usize,
    pub(crate) install_this: (usize, usize),
    pub(crate) all_and_quit_row: usize,
    pub(crate) install_all: (usize, usize),
    pub(crate) quit: (usize, usize),
}

/// Name, description, install status, DEPENDENCIES (when requires.tsv named
/// any), and ACTIONS (dependency help, reverify, and an INTEGRATION MODE
/// toggle when the skill offers more than one). Plain, uncolored content --
/// coloring the buttons and the toggle's active segment is `render_wide`/
/// `render_narrow`'s own job at actual draw time, using `info_layout`'s
/// metadata, so this function (and the line COUNT it returns) stays simple
/// and independent of `ColorMode`.
pub(crate) fn info_lines(state: &PickerState, width: usize, unicode: bool) -> Vec<String> {
    build_info(state, width, unicode).0
}

/// Same content as `info_lines`, but returning the button/toggle row and
/// column metadata instead of the text -- see `InfoLayout`.
pub(crate) fn info_layout(state: &PickerState, width: usize, unicode: bool) -> InfoLayout {
    build_info(state, width, unicode).1
}

fn build_info(state: &PickerState, width: usize, unicode: bool) -> (Vec<String>, InfoLayout) {
    let text_width = width.min(MAX_TEXT_COLS);
    let mut lines = Vec::new();
    let mut layout = InfoLayout::default();
    let skill = &state.skills[state.cursor];
    let rule_char = BorderSet::for_unicode(unicode).horizontal;
    lines.push(pad_display(
        std::iter::repeat_n(rule_char, text_width)
            .collect::<String>()
            .as_str(),
        width,
    ));
    lines.push(pad(&skill.name, width));
    lines.push(pad("", width));
    for line in wrap(&skill.description, text_width) {
        lines.push(pad(&line, width));
    }
    lines.push(pad("", width));
    lines.push(pad("STATUS", width));
    // A skill's own requirement state (the host either has `bash`/`gh`/etc.
    // or it does not) is the same regardless of which root is being looked
    // at, so it always gets exactly one line, ahead of whichever of the two
    // shapes below follows it.
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
    if skill.per_root.len() > 1 {
        // More than one root was selected this run, and a skill's own
        // installed/version answer is resolved PER ROOT (`main.rs` computes
        // one against each), so they can genuinely differ -- opencode
        // running an older copy than claude, say. The flat two-line shape
        // below would have to pick one root to speak for all of them,
        // silently hiding that divergence; this says so explicitly instead.
        for root in &skill.per_root {
            lines.push(pad(
                &format!(
                    "  {:<14} {}",
                    root.label,
                    root_status_word(root.installed, root.version_status)
                ),
                width,
            ));
        }
    } else {
        lines.push(pad(
            &format!(
                "  installed      {}",
                if skill.installed { "yes" } else { "no" }
            ),
            width,
        ));
        // Omitted for `NotInstalled`: with nothing on disk to compare
        // against, a version line would just repeat "installed no" in
        // different words.
        if let Some(word) = version_status_word(skill.version_status) {
            lines.push(pad(&format!("  version        {word}"), width));
        }
    }
    // Every requirement this skill carries, except a tool this project
    // builds and installs itself (`is_self_provided`) -- shown here, that
    // would read as something the user needs to go source themselves, when
    // the normal release flow already takes care of it. `skill_status`'s
    // own Ok/Degraded/Blocked computation is untouched by this filter (a
    // genuinely unsupported host is still reported as blocked); only this
    // display leaves it out.
    let shown: Vec<&(crate::requirements::Requirement, bool)> = skill
        .status
        .requirements
        .iter()
        .filter(|(req, _)| !is_self_provided(req))
        .collect();
    let required: Vec<&(crate::requirements::Requirement, bool)> = shown
        .iter()
        .copied()
        .filter(|(req, _)| req.strength == crate::requirements::Strength::Hard)
        .collect();
    let recommended: Vec<&(crate::requirements::Requirement, bool)> = shown
        .iter()
        .copied()
        .filter(|(req, _)| req.strength == crate::requirements::Strength::Soft)
        .collect();
    if !required.is_empty() || !recommended.is_empty() {
        let tool_col = shown
            .iter()
            .map(|(req, _)| crate::requirements::requirement_label(req).chars().count())
            .max()
            .unwrap_or(4)
            .max("TOOL".len())
            .clamp(4, 18);
        if !required.is_empty() {
            lines.push(pad("", width));
            append_dependency_table(
                &mut lines,
                &mut layout.status_cells,
                "REQUIRED TOOLS",
                &required,
                tool_col,
                width,
                text_width,
                unicode,
            );
        }
        if !recommended.is_empty() {
            lines.push(pad("", width));
            append_dependency_table(
                &mut lines,
                &mut layout.status_cells,
                "RECOMMENDED TOOLS",
                &recommended,
                tool_col,
                width,
                text_width,
                unicode,
            );
        }
    }
    // Shown only when there is something to act on: with nothing missing
    // (including a skill with no requirements at all, vacuously "nothing
    // missing"), neither button has a purpose -- installing has nothing
    // left to do, and re-checking would just confirm the same thing this
    // pane is already showing. Side by side on one row when they DO show --
    // two adjacent buttons read as two distinct actions; stacked, they read
    // as one compound thing. "Check dependencies again" rather than
    // "reverify": plain language over a word that sounds like it does
    // something more exotic than "look at PATH again".
    if shown.iter().any(|(_, met)| !met) {
        lines.push(pad("", width));
        lines.push(pad("ACTIONS", width));
        let (actions_line, dep_hint_range, check_again_range) = action_buttons_line(width);
        layout.actions = Some(ActionButtonsLayout {
            row: lines.len(),
            dep_hint: dep_hint_range,
            check_again: check_again_range,
        });
        lines.push(actions_line);
    }
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
    lines.push(pad("", width));
    let (install_this_line, install_this_range) = install_this_button_line(width, skill.installed);
    let install_this_row = lines.len();
    lines.push(install_this_line);
    // Its own section, separate from this one skill's own details above --
    // "install/update all" and "quit" act on the WHOLE run (every selected
    // skill, not just the one currently highlighted), so grouping them
    // under the single skill's own name read as if they were about that
    // skill alone.
    lines.push(pad("", width));
    lines.push(pad("ALL SELECTED SKILLS", width));
    let selected_count = state.selected_count();
    let explain = format!(
        "All {selected_count} selected skill{} will be installed or updated.",
        if selected_count == 1 { "" } else { "s" }
    );
    for line in wrap(&explain, text_width) {
        lines.push(pad(&line, width));
    }
    let (all_quit_line, install_all_range, quit_range) = install_all_and_quit_line(width);
    let all_and_quit_row = lines.len();
    lines.push(all_quit_line);
    layout.plugin_buttons = Some(PluginButtonsLayout {
        install_this_row,
        install_this: install_this_range,
        all_and_quit_row,
        install_all: install_all_range,
        quit: quit_range,
    });
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

/// True for a tool this project builds and installs itself (currently:
/// `rjq`, a bundled per-triple binary -- see `requirements.rs`'s own doc
/// comment on the fallback ladder `tool_available` gives it). Shown in the
/// picker's own DEPENDENCIES table, it would read as something the user
/// needs to go source themselves, when the normal release flow already
/// takes care of it. `requirements::skill_status`'s own Ok/Degraded/Blocked
/// computation is untouched by this -- a host missing it (and any jq
/// fallback) is still correctly reported as blocked -- only the per-tool
/// table this module draws leaves the row out.
fn is_self_provided(req: &crate::requirements::Requirement) -> bool {
    req.group.is_none() && req.tool == "rjq"
}

/// One dependency table: a heading row spanning the full width, a column
/// header row (`TOOL`/`STATUS`/`WHY`), then one row per requirement, bordered
/// with `BorderSet`'s own rules (Unicode box-drawing when the terminal
/// supports it, plain ASCII otherwise -- the same choice every other border
/// in this crate makes). Appends every row it draws to `lines` and records
/// each data row's own status-cell column span into `status_cells`, so
/// `colorize_info_row` can color "ok" green / "missing" red at actual draw
/// time without this function (or `info_lines`'s plain content) knowing
/// about `ColorMode` at all.
#[allow(clippy::too_many_arguments)]
fn append_dependency_table(
    lines: &mut Vec<String>,
    status_cells: &mut Vec<StatusCell>,
    heading: &str,
    rows: &[&(crate::requirements::Requirement, bool)],
    tool_col: usize,
    width: usize,
    text_width: usize,
    unicode: bool,
) {
    const STATUS_COL: usize = 7; // "missing", the longer of "ok"/"missing"
    const CHROME: usize = 10; // "| " x3 + " |" x3, minus the double-counted middles: see below
    let why_col = text_width
        .saturating_sub(tool_col + STATUS_COL + CHROME)
        .max(8);
    let b = BorderSet::for_unicode(unicode);
    let v = b.vertical;
    let rule = pad_display(
        std::iter::repeat_n(b.horizontal, text_width)
            .collect::<String>()
            .as_str(),
        width,
    );

    lines.push(rule.clone());
    lines.push(pad_display(
        &format!("{v} {}{v}", pad(heading, text_width.saturating_sub(3))),
        width,
    ));
    lines.push(rule.clone());
    lines.push(pad_display(
        &table_row(v, "TOOL", "STATUS", "WHY", tool_col, STATUS_COL, why_col),
        width,
    ));
    lines.push(rule.clone());
    for (req, met) in rows {
        let label = crate::requirements::requirement_label(req);
        let status_word = if *met { "ok" } else { "missing" };
        let row = table_row(
            v,
            &label,
            status_word,
            &req.why,
            tool_col,
            STATUS_COL,
            why_col,
        );
        // "| " (2) + tool_col + " | " (3) -- the status cell's own start,
        // computed the same way `table_row` lays it out, so the two can
        // never disagree about where it landed. The border glyph itself is
        // one display column regardless of Unicode/ASCII, so this offset
        // holds either way.
        let status_start = 2 + tool_col + 3;
        status_cells.push(StatusCell {
            row: lines.len(),
            start: status_start,
            end: status_start + STATUS_COL,
            ok: *met,
        });
        lines.push(pad_display(&row, width));
    }
    lines.push(rule);
}

fn table_row(
    v: char,
    tool: &str,
    status: &str,
    why: &str,
    tool_col: usize,
    status_col: usize,
    why_col: usize,
) -> String {
    format!(
        "{v} {} {v} {} {v} {} {v}",
        pad(tool, tool_col),
        pad(status, status_col),
        pad(why, why_col)
    )
}

// "Install dependencies" rather than "Help me install dependencies": side
// by side with the other button, the two need to fit a realistic DETAILS
// pane width (a 30-column floor is possible, though this pair still
// overflows one that narrow -- the same known tradeoff wizard.rs's own
// two-button rows already accept, just with unavoidably wordier verbs than
// "Install now"/"Cancel").
const DEP_HINT_LABEL: &str = "[ Install dependencies ]";
const CHECK_AGAIN_LABEL: &str = "[ Check dependencies again ]";

/// Builds the plain (uncolored) ACTIONS row -- the two buttons side by
/// side, a gap apart -- plus each one's own 0-based content-column span,
/// the same pattern `mode_toggle_line` uses for its own segments. Padded by
/// hand rather than through `pad`, which would TRUNCATE (with an ellipsis)
/// a combination wider than `width` instead of just leaving it long --
/// `wizard.rs`'s own `two_button_line` uses the identical pattern for the
/// identical reason.
fn action_buttons_line(width: usize) -> (String, (usize, usize), (usize, usize)) {
    let mut line = String::new();
    let dep_start = line.chars().count();
    line.push_str(DEP_HINT_LABEL);
    let dep_end = line.chars().count();
    line.push_str("  ");
    let chk_start = line.chars().count();
    line.push_str(CHECK_AGAIN_LABEL);
    let chk_end = line.chars().count();
    let trailing = " ".repeat(width.saturating_sub(chk_end));
    (
        format!("{line}{trailing}"),
        (dep_start, dep_end),
        (chk_start, chk_end),
    )
}

const INSTALL_THIS_LABEL: &str = "[ Install this skill ]";
const UPDATE_THIS_LABEL: &str = "[ Update this skill ]";
const INSTALL_ALL_LABEL: &str = "[ Install/update all ]";
const QUIT_LABEL: &str = "[ Quit ]";

/// The single-button "install/update this skill" row -- its own line
/// (rather than sharing one with the pair below it) since the mockup this
/// was built from gives it that weight, and because a narrow DETAILS pane
/// that cannot fit either label next to anything else at least still fits
/// it alone. The verb itself follows `installed`: "Install" when nothing is
/// on disk yet, "Update" once it is -- `state.install_only_cursor` (the
/// action this button fires) runs the same install either way, but the
/// WORD should say what it will actually feel like to the person clicking
/// it, not use one verb for both a fresh install and a reinstall of
/// something already there. Hand-padded for the same truncate-with-ellipsis
/// reason `action_buttons_line` already documents.
fn install_this_button_line(width: usize, installed: bool) -> (String, (usize, usize)) {
    let label = if installed {
        UPDATE_THIS_LABEL
    } else {
        INSTALL_THIS_LABEL
    };
    let end = label.chars().count();
    let trailing = " ".repeat(width.saturating_sub(end));
    (format!("{label}{trailing}"), (0, end))
}

/// The bulk "install/update all" and "quit" buttons, side by side -- the
/// same two-buttons-one-row shape `action_buttons_line` uses, for the same
/// "two adjacent buttons read as two distinct actions" reason.
fn install_all_and_quit_line(width: usize) -> (String, (usize, usize), (usize, usize)) {
    let mut line = String::new();
    let all_start = line.chars().count();
    line.push_str(INSTALL_ALL_LABEL);
    let all_end = line.chars().count();
    line.push_str("  ");
    let quit_start = line.chars().count();
    line.push_str(QUIT_LABEL);
    let quit_end = line.chars().count();
    let trailing = " ".repeat(width.saturating_sub(quit_end));
    (
        format!("{line}{trailing}"),
        (all_start, all_end),
        (quit_start, quit_end),
    )
}

/// Applies `InfoLayout`'s coloring to one already-built, already-padded
/// plain row: the two ACTIONS buttons each get their own full-background
/// color block, the toggle row gets only its ACTIVE segment colored
/// (`mode_active_bg`, the inactive segment(s) stay plain so it reads as a
/// switch rather than two equally weighted buttons), and a dependency
/// table's own status cell gets colored text (green "ok", red "missing")
/// rather than a background block, since it is a table value, not a
/// control. Any other row passes through unchanged.
///
/// `info_focus` is `PickerState.info_focus`'s own `(row, col)` into the
/// same focusable-control grid `mod::info_focus_rows` builds -- reverse
/// video (via `colorize_button`'s own `focused` flag) marks whichever
/// control it names, the same "this is where keyboard input lands" treatment
/// the skill list's own cursor row already gets, since a button a person
/// tabbed to but cannot SEE is not meaningfully navigable. Grid rows are
/// counted here in the exact order `info_focus_rows` builds them (ACTIONS,
/// then the mode toggle, then the plugin buttons' own two rows) -- the two
/// must agree, or a focus a keypress moved would get drawn on the wrong row.
fn colorize_info_row(
    plain: String,
    row_index: usize,
    info_layout: &InfoLayout,
    active_mode: &str,
    mode: ColorMode,
    info_focus: Option<(usize, usize)>,
) -> String {
    if mode == ColorMode::None {
        return plain;
    }
    let mut grid_row = 0;
    if let Some(actions) = &info_layout.actions {
        if actions.row == row_index {
            let focused_col = info_focus.filter(|(r, _)| *r == grid_row).map(|(_, c)| c);
            return colorize_action_buttons_row(&plain, actions, mode, focused_col);
        }
        grid_row += 1;
    }
    if let Some(toggle) = &info_layout.mode_toggle {
        if toggle.row == row_index {
            let focused_col = info_focus.filter(|(r, _)| *r == grid_row).map(|(_, c)| c);
            return colorize_mode_toggle_row(
                &plain,
                &toggle.segments,
                active_mode,
                mode,
                focused_col,
            );
        }
        grid_row += 1;
    }
    if let Some(cell) = info_layout
        .status_cells
        .iter()
        .find(|cell| cell.row == row_index)
    {
        return colorize_status_cell(&plain, cell.start, cell.end, cell.ok, mode);
    }
    if let Some(buttons) = &info_layout.plugin_buttons {
        if buttons.install_this_row == row_index {
            let focused = info_focus == Some((grid_row, 0));
            return colorize_segment(
                &plain,
                buttons.install_this.0,
                buttons.install_this.1,
                INSTALL_THIS_BUTTON_BG,
                mode,
                focused,
            );
        }
        grid_row += 1;
        if buttons.all_and_quit_row == row_index {
            let focused_col = info_focus.filter(|(r, _)| *r == grid_row).map(|(_, c)| c);
            return colorize_install_all_and_quit_row(&plain, buttons, mode, focused_col);
        }
    }
    plain
}

/// Colors just `[start, end)` of an already-built, already-padded line with
/// `bg` as a button background, leaving the rest (padding, gaps) plain --
/// the single-segment case `colorize_action_buttons_row` and
/// `colorize_install_all_and_quit_row` each repeat for their own two
/// segments.
fn colorize_segment(
    plain: &str,
    start: usize,
    end: usize,
    bg: (u8, u8, u8),
    mode: ColorMode,
    focused: bool,
) -> String {
    let chars: Vec<char> = plain.chars().collect();
    let mut out = String::new();
    out.extend(chars[..start].iter());
    let seg: String = chars[start..end].iter().collect();
    out.push_str(&colorize_button(mode, &seg, bg, focused));
    out.extend(chars[end..].iter());
    out
}

/// `focused_col`: `Some(0)` marks "Install dependencies", `Some(1)` marks
/// "Check dependencies again" -- the column index `info_focus_rows` gives
/// each within this row.
fn colorize_action_buttons_row(
    plain: &str,
    actions: &ActionButtonsLayout,
    mode: ColorMode,
    focused_col: Option<usize>,
) -> String {
    let chars: Vec<char> = plain.chars().collect();
    let mut out = String::new();
    out.extend(chars[..actions.dep_hint.0].iter());
    let dep_seg: String = chars[actions.dep_hint.0..actions.dep_hint.1]
        .iter()
        .collect();
    out.push_str(&colorize_button(
        mode,
        &dep_seg,
        DEP_HINT_BUTTON_BG,
        focused_col == Some(0),
    ));
    out.extend(chars[actions.dep_hint.1..actions.check_again.0].iter());
    let chk_seg: String = chars[actions.check_again.0..actions.check_again.1]
        .iter()
        .collect();
    out.push_str(&colorize_button(
        mode,
        &chk_seg,
        CHECK_AGAIN_BUTTON_BG,
        focused_col == Some(1),
    ));
    out.extend(chars[actions.check_again.1..].iter());
    out
}

/// `focused_col`: `Some(0)` marks "Install/update all", `Some(1)` marks
/// "Quit", the same column-index convention `colorize_action_buttons_row`
/// uses for its own pair.
fn colorize_install_all_and_quit_row(
    plain: &str,
    buttons: &PluginButtonsLayout,
    mode: ColorMode,
    focused_col: Option<usize>,
) -> String {
    let chars: Vec<char> = plain.chars().collect();
    let mut out = String::new();
    out.extend(chars[..buttons.install_all.0].iter());
    let all_seg: String = chars[buttons.install_all.0..buttons.install_all.1]
        .iter()
        .collect();
    out.push_str(&colorize_button(
        mode,
        &all_seg,
        INSTALL_ALL_BUTTON_BG,
        focused_col == Some(0),
    ));
    out.extend(chars[buttons.install_all.1..buttons.quit.0].iter());
    let quit_seg: String = chars[buttons.quit.0..buttons.quit.1].iter().collect();
    out.push_str(&colorize_button(
        mode,
        &quit_seg,
        QUIT_BUTTON_BG,
        focused_col == Some(1),
    ));
    out.extend(chars[buttons.quit.1..].iter());
    out
}

fn colorize_status_cell(
    plain: &str,
    start: usize,
    end: usize,
    ok: bool,
    mode: ColorMode,
) -> String {
    let chars: Vec<char> = plain.chars().collect();
    let seg: String = chars[start..end].iter().collect();
    let fg = if ok {
        DEPENDENCY_OK_FG
    } else {
        DEPENDENCY_MISSING_FG
    };
    let colored = colorize_text(mode, &seg, fg);
    let mut out = String::new();
    out.extend(chars[..start].iter());
    out.push_str(&colored);
    out.extend(chars[end..].iter());
    out
}

/// `focused_col`: an index into `segments` -- the keyboard-focused segment
/// gets reverse video regardless of whether it is also the ACTIVE mode;
/// `colorize_button`'s own `focused` flag already wins outright over a
/// background color, so an inactive-but-focused segment does not need its
/// own background to look selected.
fn colorize_mode_toggle_row(
    plain: &str,
    segments: &[(String, usize, usize)],
    active: &str,
    mode: ColorMode,
    focused_col: Option<usize>,
) -> String {
    let chars: Vec<char> = plain.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    for (index, (mode_name, start, end)) in segments.iter().enumerate() {
        out.extend(chars[i..*start].iter());
        let seg_text: String = chars[*start..*end].iter().collect();
        let focused = focused_col == Some(index);
        if mode_name == active || focused {
            out.push_str(&colorize_button(
                mode,
                &seg_text,
                mode_active_bg(mode_name),
                focused,
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
    let info = info_lines(state, layout.right_w, layout.unicode_borders);
    let info_meta = info_layout(state, layout.right_w, layout.unicode_borders);
    let active_mode = state.skills[state.cursor].mode.as_str();
    for body in 0..layout.body_rows {
        let info_index = body + state.info_scroll;
        let info_cell = info
            .get(info_index)
            .cloned()
            .unwrap_or_else(|| pad("", layout.right_w));
        let info_cell = colorize_info_row(
            info_cell,
            info_index,
            &info_meta,
            active_mode,
            color_mode,
            state.info_focus,
        );
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
        info_lines(state, layout.left_w, layout.unicode_borders)
    } else {
        Vec::new()
    };
    let info_meta = if state.focus == Focus::Info {
        info_layout(state, layout.left_w, layout.unicode_borders)
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
                state.info_focus,
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
                version_status: crate::cli_mode::VersionStatus::NotInstalled,
                per_root: Vec::new(),
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

    fn requirement(
        tool: &str,
        strength: crate::requirements::Strength,
        why: &str,
    ) -> crate::requirements::Requirement {
        crate::requirements::Requirement {
            tool: tool.to_string(),
            group: None,
            strength,
            why: why.to_string(),
        }
    }

    #[test]
    fn a_not_installed_skill_shows_no_version_line_at_all() {
        let state = PickerState::new(skills(&["todo"])); // installed: false by default
        let lines = info_lines(&state, 60, false);
        assert!(!lines.iter().any(|l| l.contains("version")));
    }

    #[test]
    fn an_installed_skill_shows_its_version_status_in_plain_words() {
        use crate::cli_mode::VersionStatus;
        let cases = [
            (VersionStatus::UpToDate, "up to date"),
            (VersionStatus::WouldUpdate, "would update"),
            (
                VersionStatus::Unknown,
                "unknown (installed before version tracking)",
            ),
        ];
        for (status, expected_word) in cases {
            let mut list = skills(&["todo"]);
            list[0].installed = true;
            list[0].version_status = status;
            let state = PickerState::new(list);
            let lines = info_lines(&state, 60, false);
            assert!(
                lines
                    .iter()
                    .any(|l| l.contains("version") && l.contains(expected_word)),
                "{status:?} should show {expected_word:?}: {lines:?}"
            );
        }
    }

    #[test]
    fn rjq_is_excluded_from_the_dependency_table_bash_is_shown() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![
            (
                requirement("bash", crate::requirements::Strength::Hard, "runs it"),
                true,
            ),
            (
                requirement("rjq", crate::requirements::Strength::Hard, "parses json"),
                true,
            ),
        ];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        assert!(lines.iter().any(|l| l.contains("bash")));
        assert!(
            !lines.iter().any(|l| l.contains("rjq")),
            "rjq (self-provided) should not appear in the dependency table: {lines:?}"
        );
    }

    #[test]
    fn dependencies_render_as_required_and_recommended_tables() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![
            (
                requirement(
                    "bash",
                    crate::requirements::Strength::Hard,
                    "runs the script",
                ),
                true,
            ),
            (
                requirement(
                    "gh",
                    crate::requirements::Strength::Soft,
                    "reads github CI results",
                ),
                false,
            ),
        ];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        let joined = lines.join("\n");
        assert!(joined.contains("REQUIRED TOOLS"));
        assert!(joined.contains("RECOMMENDED TOOLS"));
        assert!(joined.contains("| bash"));
        assert!(joined.contains("| gh"));
        assert!(joined.contains("ok"));
        assert!(joined.contains("missing"));
        assert!(joined.contains("runs the script"));
        assert!(joined.contains("reads github CI results"));
    }

    #[test]
    fn a_skill_with_no_requirements_shown_has_no_dependency_table() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![(
            requirement("rjq", crate::requirements::Strength::Hard, "parses json"),
            true,
        )];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        assert!(!lines.iter().any(|l| l.contains("REQUIRED TOOLS")));
        assert!(!lines.iter().any(|l| l.contains("RECOMMENDED TOOLS")));
    }

    #[test]
    fn a_missing_status_cell_is_colored_red_a_met_one_green() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![
            (
                requirement("bash", crate::requirements::Strength::Hard, "runs it"),
                true,
            ),
            (
                requirement(
                    "gh",
                    crate::requirements::Strength::Soft,
                    "reads CI results",
                ),
                false,
            ),
        ];
        let state = PickerState::new(list);
        let layout = layout_for(90, 30, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::TrueColor);
        let met_row = frame
            .iter()
            .find(|l| l.contains("bash") && l.contains("\x1b[38;2;60;180;90m"))
            .expect("a green-colored ok row for bash");
        assert!(met_row.contains("ok"));
        let missing_row = frame
            .iter()
            .find(|l| l.contains("gh") && l.contains("\x1b[38;2;210;80;80m"))
            .expect("a red-colored missing row for gh");
        assert!(missing_row.contains("missing"));
    }

    #[test]
    fn actions_are_hidden_when_nothing_is_missing() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![(
            requirement("bash", crate::requirements::Strength::Hard, "runs it"),
            true,
        )];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        assert!(
            !lines.iter().any(|l| l.contains("ACTIONS")),
            "neither button has a purpose with nothing missing: {lines:?}"
        );
        let layout = info_layout(&state, 70, false);
        assert!(layout.actions.is_none());
    }

    #[test]
    fn actions_are_hidden_for_a_skill_with_no_requirements_at_all() {
        // Vacuously "nothing missing": no requirements at all means
        // neither button has anything to act on either.
        let state = PickerState::new(skills(&["todo"]));
        let lines = info_lines(&state, 70, false);
        assert!(!lines.iter().any(|l| l.contains("ACTIONS")));
    }

    #[test]
    fn actions_show_when_something_is_missing() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![(
            requirement("bash", crate::requirements::Strength::Hard, "runs it"),
            false,
        )];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        let layout = info_layout(&state, 70, false);
        assert!(layout.actions.is_some());
    }

    #[test]
    fn the_dependency_table_draws_unicode_borders_when_asked() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![(
            requirement("bash", crate::requirements::Strength::Hard, "runs it"),
            true,
        )];
        let state = PickerState::new(list);
        let unicode_lines = info_lines(&state, 70, true);
        let joined = unicode_lines.join("\n");
        assert!(joined.contains('─'));
        assert!(joined.contains('│'));
        // "ci-failures" (the skill's own name) legitimately contains a
        // hyphen, so this checks for an ASCII RULE specifically (a run of
        // several dashes), not the mere presence of one.
        assert!(
            !joined.contains("---"),
            "expected no ASCII rule lines: {joined:?}"
        );
        let ascii_lines = info_lines(&state, 70, false);
        let ascii_joined = ascii_lines.join("\n");
        assert!(!ascii_joined.contains('─'));
        assert!(ascii_joined.contains("---"));
    }

    #[test]
    fn unicode_dependency_table_rows_are_still_exactly_the_pane_width() {
        let mut list = skills(&["ci-failures"]);
        list[0].status.requirements = vec![(
            requirement("bash", crate::requirements::Strength::Hard, "runs it"),
            true,
        )];
        let state = PickerState::new(list);
        let width = 70;
        let lines = info_lines(&state, width, true);
        for line in &lines {
            assert_eq!(line.chars().count(), width, "line was: {line:?}");
        }
    }

    #[test]
    fn a_skill_offering_more_than_one_mode_shows_the_toggle() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "mcp".to_string();
        list[0].status.requirements = vec![(
            requirement("bash", crate::requirements::Strength::Hard, "runs it"),
            false,
        )];
        let state = PickerState::new(list);
        let width = 60;
        let lines = info_lines(&state, width, false);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines.iter().any(|l| l.contains("INTEGRATION MODE")));
        assert!(lines
            .iter()
            .any(|l| l.contains("[ skill ]") && l.contains("[ mcp ]")));
    }

    #[test]
    fn a_skill_with_one_mode_still_shows_actions_but_no_toggle() {
        let mut list = skills(&["todo"]);
        list[0].status.requirements = vec![(
            requirement("bash", crate::requirements::Strength::Hard, "runs it"),
            false,
        )];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 60, false);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines.iter().any(|l| l.contains("[ Install dependencies ]")));
        assert!(lines
            .iter()
            .any(|l| l.contains("[ Check dependencies again ]")));
        assert!(!lines.iter().any(|l| l.contains("INTEGRATION MODE")));
    }

    #[test]
    fn info_layout_locates_the_action_buttons_and_the_toggles_segments() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "skill".to_string();
        list[0].status.requirements = vec![(
            requirement("bash", crate::requirements::Strength::Hard, "runs it"),
            false,
        )];
        let state = PickerState::new(list);
        let width = 60;
        let lines = info_lines(&state, width, false);
        let layout = info_layout(&state, width, false);
        let actions = layout.actions.expect("actions");
        let dep_slice: String = lines[actions.row]
            .chars()
            .skip(actions.dep_hint.0)
            .take(actions.dep_hint.1 - actions.dep_hint.0)
            .collect();
        assert_eq!(dep_slice, "[ Install dependencies ]");
        let chk_slice: String = lines[actions.row]
            .chars()
            .skip(actions.check_again.0)
            .take(actions.check_again.1 - actions.check_again.0)
            .collect();
        assert_eq!(chk_slice, "[ Check dependencies again ]");
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
        let layout = info_layout(&state, width, false);
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
        let before = info_lines(&state, 60, false);
        assert!(!before.iter().any(|l| l.contains("OUTPUT")));
        state.dep_hint();
        let after = info_lines(&state, 60, false);
        assert!(after.iter().any(|l| l.contains("OUTPUT")));
    }

    #[test]
    fn a_single_selected_root_still_shows_the_flat_installed_and_version_lines() {
        use crate::ui::model::SkillRootStatus;
        let mut list = skills(&["todo"]);
        list[0].installed = true;
        list[0].version_status = crate::cli_mode::VersionStatus::UpToDate;
        list[0].per_root = vec![SkillRootStatus {
            label: "claude".to_string(),
            installed: true,
            version_status: crate::cli_mode::VersionStatus::UpToDate,
        }];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        assert!(lines.iter().any(|l| l.contains("installed      yes")));
        assert!(lines
            .iter()
            .any(|l| l.contains("version        up to date")));
        assert!(!lines.iter().any(|l| l.contains("claude")));
    }

    #[test]
    fn more_than_one_selected_root_breaks_out_each_roots_own_status() {
        use crate::cli_mode::VersionStatus;
        use crate::ui::model::SkillRootStatus;
        let mut list = skills(&["todo"]);
        list[0].per_root = vec![
            SkillRootStatus {
                label: "claude".to_string(),
                installed: true,
                version_status: VersionStatus::UpToDate,
            },
            SkillRootStatus {
                label: "opencode".to_string(),
                installed: true,
                version_status: VersionStatus::WouldUpdate,
            },
            SkillRootStatus {
                label: "codex".to_string(),
                installed: false,
                version_status: VersionStatus::NotInstalled,
            },
        ];
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        assert!(lines
            .iter()
            .any(|l| l.contains("claude") && l.contains("installed, up to date")));
        assert!(lines
            .iter()
            .any(|l| l.contains("opencode") && l.contains("installed, would update")));
        assert!(lines
            .iter()
            .any(|l| l.contains("codex") && l.contains("not installed")));
        // The flat shape (a bare "installed"/"version" pair with no root
        // name) must not also appear once the breakdown has taken over --
        // checked as a line PREFIX, since `pad`'s own trailing spaces would
        // otherwise make a plain `contains("installed ")` match the
        // "codex" row's own "not installed" (itself followed by padding).
        assert!(!lines
            .iter()
            .any(|l| l.trim_start().starts_with("installed ")));
        assert!(!lines.iter().any(|l| l.trim_start().starts_with("version ")));
    }

    #[test]
    fn plugin_buttons_are_always_present_even_with_nothing_missing_and_no_modes() {
        // "todo" here has no requirements and no offered modes -- the case
        // where ACTIONS and INTEGRATION MODE both stay hidden -- yet the
        // three plugin buttons still show: they are global controls, not
        // conditional on this skill's own dependency state.
        let state = PickerState::new(skills(&["todo"]));
        let lines = info_lines(&state, 70, false);
        assert!(lines.iter().any(|l| l.contains("[ Install this skill ]")));
        assert!(lines.iter().any(|l| l.contains("[ Install/update all ]")));
        assert!(lines.iter().any(|l| l.contains("[ Quit ]")));
        let layout = info_layout(&state, 70, false);
        assert!(layout.plugin_buttons.is_some());
    }

    #[test]
    fn the_bulk_actions_get_their_own_section_naming_the_selected_count() {
        // Two skills, both selected (the default) -- the explanatory text
        // names the count, not just "all of them", so a change in selection
        // is visible here too, not only in the title bar.
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let lines = info_lines(&state, 70, false);
        assert!(lines.iter().any(|l| l.contains("ALL SELECTED SKILLS")));
        assert!(lines
            .iter()
            .any(|l| l.contains("All 2 selected skills will be installed or updated.")));
        // The section header must come AFTER "Install this skill" and
        // BEFORE the all/quit row -- a reader scanning top to bottom should
        // meet the per-skill button, then the section boundary, then the
        // bulk actions, not have the two interleaved.
        let this_idx = lines
            .iter()
            .position(|l| l.contains("[ Install this skill ]"))
            .unwrap();
        let header_idx = lines
            .iter()
            .position(|l| l.contains("ALL SELECTED SKILLS"))
            .unwrap();
        let all_quit_idx = lines
            .iter()
            .position(|l| l.contains("[ Install/update all ]"))
            .unwrap();
        assert!(this_idx < header_idx && header_idx < all_quit_idx);
    }

    #[test]
    fn the_install_this_button_says_update_once_the_skill_is_installed() {
        let mut list = skills(&["todo"]);
        list[0].installed = true;
        let state = PickerState::new(list);
        let lines = info_lines(&state, 70, false);
        assert!(lines.iter().any(|l| l.contains("[ Update this skill ]")));
        assert!(!lines.iter().any(|l| l.contains("Install this skill")));
    }

    #[test]
    fn info_layout_locates_the_plugin_buttons_segments() {
        let state = PickerState::new(skills(&["todo"]));
        let width = 70;
        let lines = info_lines(&state, width, false);
        let layout = info_layout(&state, width, false);
        let buttons = layout.plugin_buttons.expect("plugin_buttons");
        let this_slice: String = lines[buttons.install_this_row]
            .chars()
            .skip(buttons.install_this.0)
            .take(buttons.install_this.1 - buttons.install_this.0)
            .collect();
        assert_eq!(this_slice, "[ Install this skill ]");
        let all_slice: String = lines[buttons.all_and_quit_row]
            .chars()
            .skip(buttons.install_all.0)
            .take(buttons.install_all.1 - buttons.install_all.0)
            .collect();
        assert_eq!(all_slice, "[ Install/update all ]");
        let quit_slice: String = lines[buttons.all_and_quit_row]
            .chars()
            .skip(buttons.quit.0)
            .take(buttons.quit.1 - buttons.quit.0)
            .collect();
        assert_eq!(quit_slice, "[ Quit ]");
    }

    #[test]
    fn each_plugin_button_gets_its_own_background_color() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::TrueColor);
        let this_row = frame
            .iter()
            .find(|l| l.contains("Install this skill") && l.contains("\x1b["))
            .expect("a colored install-this row");
        assert!(this_row.contains("\x1b[48;2;50;140;70m"));
        let all_quit_row = frame
            .iter()
            .find(|l| l.contains("Install/update all") && l.contains("\x1b["))
            .expect("a colored install-all/quit row");
        assert!(all_quit_row.contains("\x1b[48;2;170;130;40m"));
        assert!(all_quit_row.contains("\x1b[48;2;150;50;50m"));
    }

    #[test]
    fn the_keyboard_focused_plugin_button_is_reverse_video() {
        let mut state = PickerState::new(skills(&["todo"]));
        // Grid row 1 is the all-and-quit row for a skill with no ACTIONS
        // and no mode toggle (plugin buttons are the only rows); column 1
        // is "Quit".
        state.info_focus = Some((1, 1));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout, ColorMode::TrueColor);
        let quit_row = frame
            .iter()
            .find(|l| l.contains("Quit"))
            .expect("the all-and-quit row");
        assert!(
            quit_row.contains("\x1b[7m[ Quit ]"),
            "the focused Quit button should be reverse video: {quit_row:?}"
        );
        assert!(
            !quit_row.contains("\x1b[7m[ Install/update all ]"),
            "the unfocused Install/update all button should not be: {quit_row:?}"
        );
    }

    #[test]
    fn a_wide_terminal_still_caps_wrapped_text_at_eighty_columns() {
        let mut list = skills(&["a"]);
        list[0].description = "word ".repeat(40); // far longer than one line
        let state = PickerState::new(list);
        let width = 150; // a very wide DETAILS pane
        let lines = info_lines(&state, width, false);
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
