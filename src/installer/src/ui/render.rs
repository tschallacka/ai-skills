// MODE: DEV
// PACKAGE: PROD
//! Builds one frame as exactly `rows` lines of exactly `cols` display
//! cells, independent of the terminal, so it is unit-testable without a
//! live tty. ASCII-only content (skill names/descriptions are ASCII), so
//! byte length is cell width throughout; the box-drawing is plain ASCII
//! (+, -, |).
//!
//! The mascot itself is NOT drawn here: when `layout.mascot_on`, this just
//! reserves its rows as blank cells (a separator line, then `mascot::HEIGHT`
//! blank rows) in the list pane. The actual sprite is painted as a colored
//! overlay at an absolute position after this frame is drawn -- mixing SGR
//! escapes into these strings would break the "every line is exactly
//! `cols` display cells" invariant every test here checks.

use super::layout::Layout;
use super::model::{Focus, PickerState};
use super::text::{overflow, pad, wrap};
use crate::requirements::SkillState;

/// Both bars are chrome, not critical content: at a width where they don't
/// fit on one line, wrapping onto a second line is the graceful fallback,
/// and only a THIRD line -- content wrapping alone still couldn't fit --
/// gets `pad`'s `...` truncation. `layout::compute` needs the resulting
/// line count (at this same `cols`) to reserve the right number of body
/// rows before this ever renders; see its own doc comment.
const BAR_MAX_LINES: usize = 3;

pub(crate) fn title_bar_text(state: &PickerState) -> String {
    format!(
        " AI-SKILLS INSTALLER  {}/{} installed  {} selected ",
        state.installed_count(),
        state.skills.len(),
        state.selected_count()
    )
}

pub(crate) const HINT_TEXT: &str =
    " Up/Dn move  Enter/Space toggle  Tab focus  a all  n none  i install  q quit";

pub(crate) fn title_bar_lines(state: &PickerState, cols: usize) -> Vec<String> {
    overflow(&title_bar_text(state), cols, BAR_MAX_LINES)
}

pub(crate) fn hint_bar_lines(cols: usize) -> Vec<String> {
    overflow(HINT_TEXT, cols, BAR_MAX_LINES)
}

/// The state suffix is appended after the name rather than inserted before
/// it, so an Ok skill's row (the common case, and the only case in most
/// existing frame-shape tests) renders byte-identical to before this state
/// tag existed.
fn list_row(state: &PickerState, index: usize, width: usize) -> String {
    let cursor = if index == state.cursor { '>' } else { ' ' };
    let checkbox = if state.selected[index] { "[x]" } else { "[ ]" };
    let name = &state.skills[index].name;
    let suffix = match state.skills[index].status.state {
        SkillState::Ok => "",
        SkillState::Degraded => " ~",
        SkillState::Blocked => " !",
    };
    pad(&format!("{cursor}{checkbox} {name}{suffix}"), width)
}

/// Name, description, install status, DEPENDENCIES (when requires.tsv named
/// any), and ACTIONS (dependency help, reverify, and the `m` line when the
/// skill offers more than one integration mode).
pub(crate) fn info_lines(state: &PickerState, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let skill = &state.skills[state.cursor];
    lines.push(pad("-".repeat(width).as_str(), width));
    lines.push(pad(&skill.name, width));
    lines.push(pad("", width));
    for line in wrap(&skill.description, width) {
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
                width,
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
    lines.push(pad("  d  help me install dependencies", width));
    lines.push(pad("  r  reverify dependencies", width));
    if skill.offered_modes.len() > 1 {
        for line in wrap(
            &format!(
                "  m  integration mode: {}   (cycles: {})",
                skill.mode,
                skill.offered_modes.join(" ")
            ),
            width,
        ) {
            lines.push(pad(&line, width));
        }
    }
    if !state.message.is_empty() {
        lines.push(pad("", width));
        for message in &state.message {
            for line in wrap(message, width) {
                lines.push(pad(&line, width));
            }
        }
    }
    lines
}

fn top_border(layout: &Layout, focus: Focus) -> String {
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
        "+{}+{}+",
        pad_center_dash(list_label, layout.left_w),
        pad_center_dash(info_label, layout.right_w)
    )
}

fn pad_center_dash(label: &str, width: usize) -> String {
    if label.len() >= width {
        return label[..width.min(label.len())].to_string();
    }
    format!("{label}{}", "-".repeat(width - label.len()))
}

fn bottom_border(layout: &Layout) -> String {
    format!(
        "+{}+{}+",
        "-".repeat(layout.left_w),
        "-".repeat(layout.right_w)
    )
}

pub fn render_frame(state: &PickerState, layout: &Layout) -> Vec<String> {
    let mut out = Vec::with_capacity(layout.rows);
    out.extend(title_bar_lines(state, layout.cols));

    if layout.narrow {
        render_narrow(state, layout, &mut out);
    } else {
        render_wide(state, layout, &mut out);
    }
    out.extend(hint_bar_lines(layout.cols));
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
        return "-".repeat(layout.left_w);
    }
    pad("", layout.left_w)
}

fn render_wide(state: &PickerState, layout: &Layout, out: &mut Vec<String>) {
    out.push(top_border(layout, state.focus));
    let info = info_lines(state, layout.right_w);
    for body in 0..layout.body_rows {
        let info_index = body + state.info_scroll;
        let info_cell = info
            .get(info_index)
            .cloned()
            .unwrap_or_else(|| pad("", layout.right_w));
        out.push(format!("|{}|{info_cell}|", list_cell(state, layout, body)));
    }
    out.push(bottom_border(layout));
}

fn render_narrow(state: &PickerState, layout: &Layout, out: &mut Vec<String>) {
    let label = if state.focus == Focus::Info {
        "[DETAILS]"
    } else {
        "[SKILLS]"
    };
    out.push(format!("+{}+", pad_center_dash(label, layout.left_w)));
    let info = if state.focus == Focus::Info {
        info_lines(state, layout.left_w)
    } else {
        Vec::new()
    };
    for body in 0..layout.body_rows {
        let cell = if state.focus == Focus::Info {
            info.get(body + state.info_scroll)
                .cloned()
                .unwrap_or_else(|| pad("", layout.left_w))
        } else if state.scroll + body < state.skills.len() {
            list_row(state, state.scroll + body, layout.left_w)
        } else {
            pad("", layout.left_w)
        };
        out.push(format!("|{cell}|"));
    }
    out.push(format!("+{}+", "-".repeat(layout.left_w)));
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
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let title_rows = title_bar_lines(state, cols).len();
        let hint_rows = hint_bar_lines(cols).len();
        layout::compute(cols, rows, &names, color_capable, title_rows, hint_rows)
    }

    #[test]
    fn every_line_is_exactly_the_terminal_width() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let layout = layout_for(80, 24, &state, true);
        let frame = render_frame(&state, &layout);
        assert_eq!(frame.len(), layout.rows);
        for line in &frame {
            assert_eq!(line.len(), layout.cols, "line was: {line:?}");
        }
    }

    #[test]
    fn the_cursor_row_carries_the_marker() {
        let mut state = PickerState::new(skills(&["todo", "bug-report"]));
        state.cursor = 1;
        let layout = layout_for(80, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let frame = render_frame(&state, &layout);
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
        let frame = render_frame(&state, &layout);
        assert!(frame[title_rows + 1].contains("[ ] todo"));
    }

    #[test]
    fn a_narrow_terminal_renders_one_pane_with_no_pipe_divider() {
        let state = PickerState::new(skills(&["todo"]));
        let layout = layout_for(40, 24, &state, true);
        let title_rows = title_bar_lines(&state, layout.cols).len();
        let hint_rows = hint_bar_lines(layout.cols).len();
        let frame = render_frame(&state, &layout);
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
        let frame = render_frame(&state, &layout);
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
        let frame = render_frame(&state, &layout);
        // Every line still comes out exactly `cols` wide even with the
        // mascot's rows reserved -- the invariant every other test checks.
        for line in &frame {
            assert_eq!(line.len(), layout.cols);
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
    fn a_skill_offering_more_than_one_mode_shows_the_mode_line() {
        let mut list = skills(&["ai-text-editor"]);
        list[0].offered_modes = vec!["skill".to_string(), "mcp".to_string()];
        list[0].mode = "mcp".to_string();
        let state = PickerState::new(list);
        let width = 60;
        let lines = info_lines(&state, width);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines
            .iter()
            .any(|l| l.contains("integration mode: mcp") && l.contains("cycles: skill mcp")));
    }

    #[test]
    fn a_skill_with_one_mode_still_shows_actions_but_no_mode_line() {
        let state = PickerState::new(skills(&["todo"]));
        let lines = info_lines(&state, 60);
        assert!(lines.iter().any(|l| l.contains("ACTIONS")));
        assert!(lines
            .iter()
            .any(|l| l.contains("help me install dependencies")));
        assert!(lines.iter().any(|l| l.contains("reverify dependencies")));
        assert!(!lines.iter().any(|l| l.contains("integration mode")));
    }
}
