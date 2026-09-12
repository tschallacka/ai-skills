// MODE: DEV
// PACKAGE: PROD
//! T144, goal 03: a fine-grained uninstall screen for the interactive
//! picker, reachable from `run_interactive`. A new, independent module
//! rather than an extension of `model::PickerState` -- confirmed against
//! the real `ui/mod.rs::run_picker`/`model.rs` architecture (working-context.md
//! records the read): install's state is oriented entirely around
//! selection/mode-cycling of the shipped skill list, and entangling
//! uninstall-only state (a chosen already-installed skill, an
//! `UninstallPreview`) into it would couple two screens that share no
//! fields in practice. `terminal`/`input::Key`/`layout` are still reused
//! directly; only the state shape and rendering are new.

use super::input::Key;
use super::terminal;
use crate::uninstall::{self, UninstallPreview, UninstallReport};
use std::path::{Path, PathBuf};

pub struct Entry {
    pub skill: String,
    pub target_root: PathBuf,
}

enum Screen {
    List,
    Preview(UninstallPreview),
    Done(UninstallReport),
}

pub struct UninstallPickerState {
    entries: Vec<Entry>,
    cursor: usize,
    scroll: usize,
    screen: Screen,
    pub confirmed: bool,
    pub done: bool,
}

impl UninstallPickerState {
    pub fn new(entries: Vec<Entry>) -> Self {
        UninstallPickerState {
            entries,
            cursor: 0,
            scroll: 0,
            screen: Screen::List,
            confirmed: false,
            done: false,
        }
    }

    fn move_by(&mut self, delta: isize) {
        if self.entries.is_empty() {
            return;
        }
        let max = self.entries.len() - 1;
        let next = (self.cursor as isize + delta).clamp(0, max as isize);
        self.cursor = next as usize;
    }

    fn clamp_scroll(&mut self, body_rows: usize) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + body_rows {
            self.scroll = self.cursor + 1 - body_rows;
        }
    }
}

pub fn handle_key(
    state: &mut UninstallPickerState,
    key: Key,
    source_root: &Path,
    home: &Path,
    kind: Option<&str>,
) {
    match &state.screen {
        Screen::List => match key {
            Key::Up | Key::Char('k') => state.move_by(-1),
            Key::Down | Key::Char('j') => state.move_by(1),
            Key::Enter | Key::Space => {
                if let Some(entry) = state.entries.get(state.cursor) {
                    let preview = uninstall::preview_uninstall(
                        source_root,
                        &entry.skill,
                        &entry.target_root,
                        home,
                        kind,
                    );
                    state.screen = Screen::Preview(preview);
                }
            }
            Key::Char('q') | Key::Escape => {
                state.done = true;
                state.confirmed = false;
            }
            _ => {}
        },
        Screen::Preview(_) => match key {
            Key::Enter | Key::Char('y') => {
                if let Some(entry) = state.entries.get(state.cursor) {
                    if let Ok(report) = uninstall::uninstall_skill(
                        source_root,
                        &entry.skill,
                        &entry.target_root,
                        home,
                        kind,
                    ) {
                        state.confirmed = report.was_installed;
                        state.screen = Screen::Done(report);
                    }
                }
            }
            Key::Char('q') | Key::Escape => {
                state.screen = Screen::List;
            }
            _ => {}
        },
        Screen::Done(_) => {
            state.done = true;
        }
    }
}

const LIST_HINT: &str = " Up/Dn move  Enter preview  q back";
const PREVIEW_HINT: &str = " Enter/y confirm removal  q/Esc cancel";

pub fn render_frame(state: &UninstallPickerState, cols: usize, rows: usize) -> Vec<String> {
    let body_rows = rows.saturating_sub(3).max(1);
    match &state.screen {
        Screen::List => render_list(state, cols, body_rows),
        Screen::Preview(preview) => render_preview(state, preview, cols, body_rows),
        Screen::Done(report) => render_done(report, cols, body_rows),
    }
}

fn frame(title: &str, hint: &str, body: Vec<String>, cols: usize, body_rows: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(body_rows + 3);
    out.push(super::render::pad(title, cols));
    out.push("-".repeat(cols));
    let mut body = body;
    body.truncate(body_rows);
    while body.len() < body_rows {
        body.push(String::new());
    }
    for line in body {
        out.push(super::render::pad(&line, cols));
    }
    out.push(super::render::pad(hint, cols));
    out
}

fn render_list(state: &UninstallPickerState, cols: usize, body_rows: usize) -> Vec<String> {
    let mut clamped = UninstallPickerState {
        entries: Vec::new(),
        cursor: state.cursor,
        scroll: state.scroll,
        screen: Screen::List,
        confirmed: false,
        done: false,
    };
    clamped.clamp_scroll(body_rows);
    let scroll = clamped.scroll;

    let lines: Vec<String> = state
        .entries
        .iter()
        .enumerate()
        .skip(scroll)
        .map(|(i, e)| {
            let cursor = if i == state.cursor { '>' } else { ' ' };
            format!(
                "{cursor} {} ({})",
                e.skill,
                e.target_root.join(&e.skill).display()
            )
        })
        .collect();
    frame(
        "UNINSTALL -- choose an installed skill",
        LIST_HINT,
        lines,
        cols,
        body_rows,
    )
}

fn render_preview(
    state: &UninstallPickerState,
    preview: &UninstallPreview,
    cols: usize,
    body_rows: usize,
) -> Vec<String> {
    let entry = &state.entries[state.cursor];
    let mut lines = vec![format!(
        "Remove {} from {}?",
        entry.skill,
        entry.target_root.join(&entry.skill).display()
    )];
    if !preview.would_remove_shared_binaries.is_empty() {
        lines.push(format!(
            "  shared binaries removed: {}",
            preview.would_remove_shared_binaries.join(", ")
        ));
    }
    if !preview.would_keep_shared_binaries.is_empty() {
        lines.push(format!(
            "  shared binaries kept (needed elsewhere): {}",
            preview.would_keep_shared_binaries.join(", ")
        ));
    }
    if !preview.would_remove_plugins.is_empty() {
        lines.push(format!(
            "  companion plugins removed: {}",
            preview.would_remove_plugins.join(", ")
        ));
    }
    if preview.has_mcp_entry {
        lines.push("  MCP registration will be removed".to_string());
    }
    if !preview.modified_files.is_empty() {
        lines.push(format!(
            "  NOTE: edited since install, removed anyway: {}",
            preview.modified_files.join(", ")
        ));
    }
    frame("UNINSTALL -- preview", PREVIEW_HINT, lines, cols, body_rows)
}

fn render_done(report: &UninstallReport, cols: usize, body_rows: usize) -> Vec<String> {
    if !report.was_installed {
        return frame(
            "UNINSTALL -- done",
            " press any key to return",
            vec!["Not installed there; nothing to remove.".to_string()],
            cols,
            body_rows,
        );
    }
    let mut lines = vec!["Removed.".to_string()];
    for binary in &report.removed_shared_binaries {
        lines.push(format!("  removed shared binary: {binary}"));
    }
    for plugin in &report.removed_plugins {
        lines.push(format!("  removed companion plugin: {plugin}"));
    }
    for grant in &report.permissions_removed {
        lines.push(format!("  revoked permission grant: {grant}"));
    }
    frame(
        "UNINSTALL -- done",
        " press any key to return",
        lines,
        cols,
        body_rows,
    )
}

/// Runs the uninstall screen until the user backs out or completes a
/// removal -- same loop shape as `run_picker`.
pub fn run(entries: Vec<Entry>, source_root: &Path, home: &Path, kind: Option<&str>) {
    if !terminal::is_tty() || entries.is_empty() {
        return;
    }
    let mut state = UninstallPickerState::new(entries);
    let saved = terminal::enter();
    let rx = terminal::spawn_reader();

    loop {
        let (cols, rows) = terminal::size();
        terminal::draw(&render_frame(&state, cols, rows));
        match super::input::read_key(&rx) {
            Key::Tick => continue,
            Key::Eof => {
                state.done = true;
            }
            key => handle_key(&mut state, key, source_root, home, kind),
        }
        if state.done {
            break;
        }
    }

    terminal::leave(&saved);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(names: &[&str]) -> Vec<Entry> {
        names
            .iter()
            .map(|n| Entry {
                skill: n.to_string(),
                target_root: PathBuf::from("/tmp/root"),
            })
            .collect()
    }

    #[test]
    fn cursor_movement_clamps_to_the_entry_list_bounds() {
        let mut state = UninstallPickerState::new(entries(&["a", "b", "c"]));
        state.move_by(-1);
        assert_eq!(state.cursor, 0);
        state.move_by(10);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn enter_from_the_list_moves_into_a_preview_without_touching_disk() {
        let source_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let mut state = UninstallPickerState::new(entries(&["todo"]));

        handle_key(
            &mut state,
            Key::Enter,
            source_root.path(),
            home.path(),
            Some("claude"),
        );

        assert!(matches!(state.screen, Screen::Preview(_)));
        assert!(!state.done);
    }

    #[test]
    fn q_from_the_preview_returns_to_the_list_rather_than_exiting() {
        let source_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let mut state = UninstallPickerState::new(entries(&["todo"]));
        handle_key(
            &mut state,
            Key::Enter,
            source_root.path(),
            home.path(),
            Some("claude"),
        );

        handle_key(
            &mut state,
            Key::Char('q'),
            source_root.path(),
            home.path(),
            Some("claude"),
        );

        assert!(matches!(state.screen, Screen::List));
        assert!(!state.done);
    }

    #[test]
    fn y_on_the_preview_confirms_and_marks_done() {
        let source_root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(source_root.path().join("todo")).unwrap();
        std::fs::write(source_root.path().join("todo").join("SKILL.md"), "content").unwrap();
        let home = tempfile::tempdir().unwrap();
        let target = home.path().join(".claude/skills");
        crate::install::install_skill(source_root.path(), "todo", &target, home.path(), None, true)
            .unwrap();
        let mut state = UninstallPickerState::new(vec![Entry {
            skill: "todo".to_string(),
            target_root: target,
        }]);
        handle_key(
            &mut state,
            Key::Enter,
            source_root.path(),
            home.path(),
            Some("claude"),
        );

        handle_key(
            &mut state,
            Key::Char('y'),
            source_root.path(),
            home.path(),
            Some("claude"),
        );

        assert!(state.confirmed);
        assert!(matches!(state.screen, Screen::Done(_)));
    }
}
