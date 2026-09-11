// MODE: DEV
// PACKAGE: PROD
//! Builds one frame as exactly `rows` lines of exactly `cols` display cells,
//! independent of the terminal -- so it is unit-testable the way
//! 36-ui-render.sh's IUI_POSITION=0 headless mode is, without a live tty.
//! ASCII-only content (skill names/descriptions are ASCII in manifest.rs),
//! so byte length is cell width throughout; the box-drawing is plain ASCII
//! (+, -, |), not the Minecraft glyph set 35/36-ui-*.sh draw.
//!
//! The mascot itself is NOT drawn here: when `layout.mascot_on`, this just
//! reserves its rows as blank cells (a separator line, then `mascot::HEIGHT`
//! blank rows) in the list pane. mod.rs paints the actual sprite as a
//! colored overlay at an absolute position after this frame is drawn --
//! mixing SGR escapes into these strings would break the "every line is
//! exactly `cols` display cells" invariant every test here checks.

use super::layout::Layout;
use super::model::{Focus, PickerState};
use crate::requirements::SkillState;

fn pad(text: &str, width: usize) -> String {
    if text.len() > width {
        return if width == 0 {
            String::new()
        } else if width == 1 {
            "~".to_string()
        } else {
            format!("{}~", &text[..width - 1])
        };
    }
    format!("{text:<width$}")
}

/// Word-wraps to `width`, hyphenating a token wider than the pane -- ported
/// from installer/src/35-ui-model.sh's iui_wrap.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut lines = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.len() <= width {
            lines.push(remaining.to_string());
            break;
        }
        let candidate = &remaining[..width];
        match candidate.rfind(' ') {
            Some(split) if split > 0 => {
                lines.push(candidate[..split].to_string());
                remaining = remaining[split..].trim_start_matches(' ');
            }
            _ => {
                lines.push(format!("{}-", &remaining[..width - 1]));
                remaining = &remaining[width - 1..];
            }
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn title_bar(state: &PickerState, cols: usize) -> String {
    let text = format!(
        " AI-SKILLS INSTALLER  {}/{} installed  {} selected ",
        state.installed_count(),
        state.skills.len(),
        state.selected_count()
    );
    pad(&text, cols)
}

fn hint_bar(cols: usize) -> String {
    pad(
        " Up/Dn move  Enter/Space toggle  Tab focus  a all  n none  i install  q quit",
        cols,
    )
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

/// Name, description, install status, and (when requires.tsv named any)
/// DEPENDENCIES -- no ACTIONS pane yet (iui_info_status's `d`/`r`/`m` hint
/// carousel and integration-mode cycling stay unported until integration.tsv
/// has a Rust model).
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
            for line in wrap(&format!("  {label} ({strength}): {mark} -- {}", req.why), width) {
                lines.push(pad(&line, width));
            }
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
    out.push(title_bar(state, layout.cols));

    if layout.narrow {
        render_narrow(state, layout, &mut out);
    } else {
        render_wide(state, layout, &mut out);
    }
    out.push(hint_bar(layout.cols));
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
            })
            .collect()
    }

    #[test]
    fn every_line_is_exactly_the_terminal_width() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(80, 24, &names, true);
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
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(80, 24, &names, true);
        let frame = render_frame(&state, &layout);
        let body_line = &frame[3];
        assert!(body_line.contains(">[x] bug-report"));
    }

    #[test]
    fn a_deselected_skill_shows_an_empty_checkbox() {
        let mut state = PickerState::new(skills(&["todo"]));
        state.toggle(0);
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(80, 24, &names, true);
        let frame = render_frame(&state, &layout);
        assert!(frame[2].contains("[ ] todo"));
    }

    #[test]
    fn a_narrow_terminal_renders_one_pane_with_no_pipe_divider() {
        let state = PickerState::new(skills(&["todo"]));
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(40, 24, &names, true);
        let frame = render_frame(&state, &layout);
        for line in &frame[2..frame.len() - 2] {
            assert_eq!(line.matches('|').count(), 2, "line was: {line:?}");
        }
    }

    #[test]
    fn the_title_bar_reports_selected_and_installed_counts() {
        let mut state = PickerState::new(skills(&["a", "b", "c"]));
        state.toggle(0);
        state.skills[1].installed = true;
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(80, 24, &names, true);
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
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(80, 30, &names, true);
        assert!(layout.mascot_on);
        let frame = render_frame(&state, &layout);
        // Every line still comes out exactly `cols` wide even with the
        // mascot's rows reserved -- the invariant every other test checks.
        for line in &frame {
            assert_eq!(line.len(), layout.cols);
        }
        let separator_row = 2 + layout.list_rows;
        assert!(frame[separator_row].contains("-------"));
    }
}
