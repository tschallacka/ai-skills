// MODE: DEV
// PACKAGE: PROD
//! Builds one frame as exactly `rows` lines of exactly `cols` display cells,
//! independent of the terminal -- so it is unit-testable the way
//! 36-ui-render.sh's IUI_POSITION=0 headless mode is, without a live tty.
//! ASCII-only content (skill names/descriptions are ASCII in manifest.rs),
//! so byte length is cell width throughout; the box-drawing is plain ASCII
//! (+, -, |), not the Minecraft glyph set 35/36-ui-*.sh draw -- no mascot,
//! no palette, no per-role colour in this slice.

use super::layout::Layout;
use super::model::{Focus, PickerState};

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

fn list_row(state: &PickerState, index: usize, width: usize) -> String {
    let cursor = if index == state.cursor { '>' } else { ' ' };
    let checkbox = if state.selected[index] { "[x]" } else { "[ ]" };
    let name = &state.skills[index].name;
    pad(&format!("{cursor}{checkbox} {name}"), width)
}

/// Name, description, and install status; no dependency table or ACTIONS
/// pane yet (iui_info_status's DEPENDENCIES/ACTIONS sections stay unported
/// until runtime_requirements and integration.tsv have a Rust model).
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

fn render_wide(state: &PickerState, layout: &Layout, out: &mut Vec<String>) {
    out.push(top_border(layout, state.focus));
    let info = info_lines(state, layout.right_w);
    for body in 0..layout.body_rows {
        let list_cell = if body < state.skills.len().saturating_sub(state.scroll) {
            list_row(state, state.scroll + body, layout.left_w)
        } else {
            pad("", layout.left_w)
        };
        let info_index = body + state.info_scroll;
        let info_cell = info
            .get(info_index)
            .cloned()
            .unwrap_or_else(|| pad("", layout.right_w));
        out.push(format!("|{list_cell}|{info_cell}|"));
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
    use crate::ui::layout;
    use crate::ui::model::SkillEntry;

    fn skills(names: &[&str]) -> Vec<SkillEntry> {
        names
            .iter()
            .map(|n| SkillEntry {
                name: n.to_string(),
                description: format!("{n} does things."),
                installed: false,
            })
            .collect()
    }

    #[test]
    fn every_line_is_exactly_the_terminal_width() {
        let state = PickerState::new(skills(&["todo", "bug-report"]));
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(80, 24, &names);
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
        let layout = layout::compute(80, 24, &names);
        let frame = render_frame(&state, &layout);
        let body_line = &frame[3];
        assert!(body_line.contains(">[x] bug-report"));
    }

    #[test]
    fn a_deselected_skill_shows_an_empty_checkbox() {
        let mut state = PickerState::new(skills(&["todo"]));
        state.toggle(0);
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(80, 24, &names);
        let frame = render_frame(&state, &layout);
        assert!(frame[2].contains("[ ] todo"));
    }

    #[test]
    fn a_narrow_terminal_renders_one_pane_with_no_pipe_divider() {
        let state = PickerState::new(skills(&["todo"]));
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(40, 24, &names);
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
        let layout = layout::compute(80, 24, &names);
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
}
