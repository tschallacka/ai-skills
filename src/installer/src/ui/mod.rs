// MODE: DEV
// PACKAGE: PROD
//! The full-screen skill picker's event loop -- the seam installer/src/
//! 37-ui-input.sh calls iui_select_skills(). Ties layout, model, render,
//! input, terminal and the mascot together; see each submodule's own doc
//! comment for what it ports and what it deliberately leaves out (no
//! dependency table, no mouse, no integration-mode cycling, no hint
//! carousel or "right-top" mascot placement).

pub mod input;
pub mod layout;
pub mod mascot;
pub mod model;
pub mod render;
pub mod terminal;

use input::Key;
use mascot::{ColorMode, EyeAnimator};
use model::{PickerState, SkillEntry};

/// `None` when fd 0 is not a tty (the caller's cue to fall back to a plain
/// listing, same as install.sh's iui_run returning 69) or the user quit
/// (q/Esc/Ctrl-C/EOF); `Some(names)` in original skill order once `i`
/// confirms.
pub fn run_picker(skills: Vec<SkillEntry>) -> Option<Vec<String>> {
    if !terminal::is_tty() {
        return None;
    }
    let mut state = PickerState::new(skills);
    let saved = terminal::enter();
    let rx = terminal::spawn_reader();
    // Probed once, same as install.sh's COLOR_MODE cache: the picker
    // redraws on every keypress and tick, and a per-frame `tput` shellout
    // would be one process spawn per second at minimum.
    let color_mode = mascot::detect_color_mode();
    let mut eyes = EyeAnimator::new();

    loop {
        let (cols, rows) = terminal::size();
        let names: Vec<&str> = state.skills.iter().map(|s| s.name.as_str()).collect();
        let layout = layout::compute(cols, rows, &names, color_mode != ColorMode::None);
        state.clamp_scroll(layout.body_rows);
        clamp_info_scroll(&mut state, &layout);
        terminal::draw(&render::render_frame(&state, &layout));
        if layout.mascot_on {
            draw_mascot(&layout, color_mode, eyes.current());
        }

        match input::read_key(&rx) {
            Key::Tick => {
                eyes.advance();
                continue;
            }
            Key::Eof => {
                state.done = true;
                state.confirmed = false;
            }
            key => handle_key(&mut state, key, &layout),
        }
        if state.done {
            break;
        }
    }

    terminal::leave(&saved);
    if state.confirmed {
        Some(state.selected_names())
    } else {
        None
    }
}

/// The sprite sits under the list, left-aligned two columns in (past the
/// pane's leading `|`), starting right below the separator render.rs left
/// blank at body row `layout.list_rows` -- title(1) + top border(1) + that
/// separator's own row + 1 = `list_rows + 4` in absolute terminal rows.
fn draw_mascot(layout: &layout::Layout, mode: ColorMode, eye: mascot::EyeState) {
    let lines: Vec<String> = (0..mascot::HEIGHT)
        .map(|row| mascot::head_line(mode, row, eye))
        .collect();
    terminal::draw_overlay(layout.list_rows + 4, 2, &lines);
}

/// The info pane's own line count depends on the current skill's
/// description length, so its max scroll is recomputed every frame rather
/// than tracked as separate state -- same reasoning as
/// 35-ui-model.sh's iui_clamp_info_scroll.
fn clamp_info_scroll(state: &mut PickerState, layout: &layout::Layout) {
    let width = if layout.narrow {
        layout.left_w
    } else {
        layout.right_w
    };
    let total = render::info_lines(state, width).len();
    let max_scroll = total.saturating_sub(layout.body_rows);
    if state.info_scroll > max_scroll {
        state.info_scroll = max_scroll;
    }
}

fn handle_key(state: &mut PickerState, key: Key, layout: &layout::Layout) {
    match key {
        Key::Up | Key::Char('k') => state.move_by(-1),
        Key::Down | Key::Char('j') => state.move_by(1),
        Key::PageUp => state.move_by(-(layout.body_rows as isize)),
        Key::PageDown => state.move_by(layout.body_rows as isize),
        Key::Home => state.go_home(),
        Key::End => {
            let width = if layout.narrow {
                layout.left_w
            } else {
                layout.right_w
            };
            let max_scroll = render::info_lines(state, width)
                .len()
                .saturating_sub(layout.body_rows);
            state.go_end(max_scroll);
        }
        Key::Enter | Key::Space => state.toggle(state.cursor),
        Key::Tab | Key::ShiftTab => state.toggle_focus(),
        Key::Char('a') => state.select_all(),
        Key::Char('n') => state.select_none(),
        Key::Char('i') => {
            state.done = true;
            state.confirmed = true;
        }
        Key::Char('q') | Key::Escape => {
            state.done = true;
            state.confirmed = false;
        }
        _ => {}
    }
}
