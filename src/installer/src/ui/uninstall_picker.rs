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
use super::text::{overflow, pad, wrap};
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

const LIST_TITLE: &str = "UNINSTALL -- choose an installed skill";
const LIST_HINT: &str = " Up/Dn move  Enter preview  q back";
const PREVIEW_TITLE: &str = "UNINSTALL -- preview";
const PREVIEW_HINT: &str = " Enter/y confirm removal  q/Esc cancel";
const DONE_TITLE: &str = "UNINSTALL -- done";
const DONE_HINT: &str = " press any key to return";

/// Chrome, not critical content -- same reasoning as the main picker's
/// title/hint bars (`render::BAR_MAX_LINES`): wrap onto extra lines first,
/// and truncate (`pad`'s `...`) only the last of them if it still doesn't
/// fit.
const BAR_MAX_LINES: usize = 3;

pub fn render_frame(state: &UninstallPickerState, cols: usize, rows: usize) -> Vec<String> {
    let base_body_rows = rows.saturating_sub(3).max(1);
    match &state.screen {
        Screen::List => {
            let body_rows = usable_body_rows(LIST_TITLE, LIST_HINT, cols, base_body_rows);
            render_list(state, cols, body_rows)
        }
        Screen::Preview(preview) => {
            let body_rows = usable_body_rows(PREVIEW_TITLE, PREVIEW_HINT, cols, base_body_rows);
            render_preview(state, preview, cols, body_rows)
        }
        Screen::Done(report) => {
            let body_rows = usable_body_rows(DONE_TITLE, DONE_HINT, cols, base_body_rows);
            render_done(report, cols, body_rows)
        }
    }
}

/// However many extra lines the title or hint bar needs beyond one each
/// (rare -- both are short fixed strings -- but possible at an extreme
/// width) comes out of the body, computed once here so the scroll-clamping
/// that runs before `frame` is built agrees with what `frame` actually has
/// room to draw.
fn usable_body_rows(title: &str, hint: &str, cols: usize, base_body_rows: usize) -> usize {
    let title_rows = overflow(title, cols, BAR_MAX_LINES).len();
    let hint_rows = overflow(hint, cols, BAR_MAX_LINES).len();
    let extra = (title_rows - 1) + (hint_rows - 1);
    base_body_rows.saturating_sub(extra).max(1)
}

fn frame(title: &str, hint: &str, body: Vec<String>, cols: usize, body_rows: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(body_rows + 3);
    out.extend(overflow(title, cols, BAR_MAX_LINES));
    out.push("-".repeat(cols));
    let mut body = body;
    body.truncate(body_rows);
    while body.len() < body_rows {
        body.push(String::new());
    }
    for line in body {
        out.push(pad(&line, cols));
    }
    out.extend(overflow(hint, cols, BAR_MAX_LINES));
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
            list_row(
                cursor,
                &e.skill,
                &e.target_root.join(&e.skill).display().to_string(),
                cols,
            )
        })
        .collect();
    frame(LIST_TITLE, LIST_HINT, lines, cols, body_rows)
}

/// The skill name is what tells two entries apart; the path after it is
/// useful but secondary, and the one place a real path truncated to
/// illegibility in review (B<pending>). So when the whole row doesn't fit,
/// only the parenthesized path is ellipsized -- the name is never cut.
fn list_row(cursor: char, skill: &str, path: &str, cols: usize) -> String {
    let fixed = 5 + skill.len(); // "C SKILL (" + ")" -- C=cursor+space(2), " ("=2, ")"=1
    let available = cols.saturating_sub(fixed).max(4);
    let path_display = if path.len() > available {
        pad(path, available)
    } else {
        path.to_string()
    };
    format!("{cursor} {skill} ({path_display})")
}

/// Everything on this screen names exactly what an irreversible removal is
/// about to do -- critical content, never truncated. Each logical message
/// is word-wrapped (uncapped: `wrap`, not `overflow`) rather than cut with
/// `pad`'s `...`, so the full skill name and the full target path are
/// always visible before the user confirms, however many physical lines
/// that takes.
fn render_preview(
    state: &UninstallPickerState,
    preview: &UninstallPreview,
    cols: usize,
    body_rows: usize,
) -> Vec<String> {
    let entry = &state.entries[state.cursor];
    let mut messages = vec![format!(
        "Remove {} from {}?",
        entry.skill,
        entry.target_root.join(&entry.skill).display()
    )];
    if !preview.would_remove_shared_binaries.is_empty() {
        messages.push(format!(
            "  shared binaries removed: {}",
            preview.would_remove_shared_binaries.join(", ")
        ));
    }
    if !preview.would_keep_shared_binaries.is_empty() {
        messages.push(format!(
            "  shared binaries kept (needed elsewhere): {}",
            preview.would_keep_shared_binaries.join(", ")
        ));
    }
    if !preview.would_remove_plugins.is_empty() {
        messages.push(format!(
            "  companion plugins removed: {}",
            preview.would_remove_plugins.join(", ")
        ));
    }
    if preview.has_mcp_entry {
        messages.push("  MCP registration will be removed".to_string());
    }
    if !preview.modified_files.is_empty() {
        messages.push(format!(
            "  NOTE: edited since install, removed anyway: {}",
            preview.modified_files.join(", ")
        ));
    }
    let lines: Vec<String> = messages.iter().flat_map(|m| wrap(m, cols)).collect();
    frame(PREVIEW_TITLE, PREVIEW_HINT, lines, cols, body_rows)
}

/// Reports what an already-irreversible removal actually did -- same
/// never-truncate treatment as the preview above, and for the same reason:
/// this is the one place left to see exactly what was removed.
fn render_done(report: &UninstallReport, cols: usize, body_rows: usize) -> Vec<String> {
    if !report.was_installed {
        return frame(
            DONE_TITLE,
            DONE_HINT,
            wrap("Not installed there; nothing to remove.", cols),
            cols,
            body_rows,
        );
    }
    let mut messages = vec!["Removed.".to_string()];
    for binary in &report.removed_shared_binaries {
        messages.push(format!("  removed shared binary: {binary}"));
    }
    for plugin in &report.removed_plugins {
        messages.push(format!("  removed companion plugin: {plugin}"));
    }
    for grant in &report.permissions_removed {
        messages.push(format!("  revoked permission grant: {grant}"));
    }
    let lines: Vec<String> = messages.iter().flat_map(|m| wrap(m, cols)).collect();
    frame(DONE_TITLE, DONE_HINT, lines, cols, body_rows)
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

    #[test]
    fn list_row_ellipsizes_the_path_but_never_the_skill_name() {
        let row = list_row(
            '>',
            "ai-text-editor",
            "/tmp/tmp.Bpsv1ejDt0/installed3/ai-text-editor",
            40,
        );
        assert!(row.contains("ai-text-editor"), "name was cut: {row:?}");
        assert!(row.contains("..."), "path was not truncated: {row:?}");
        assert!(!row.contains('~'));
    }

    #[test]
    fn list_row_leaves_a_short_path_untouched() {
        let row = list_row('>', "todo", "/tmp/x", 40);
        assert_eq!(row, "> todo (/tmp/x)");
    }

    #[test]
    fn the_confirm_question_is_never_truncated_however_narrow() {
        let entries = vec![Entry {
            skill: "ai-text-editor".to_string(),
            target_root: PathBuf::from("/tmp/tmp.Bpsv1ejDt0/installed3"),
        }];
        let mut state = UninstallPickerState::new(entries);
        state.screen = Screen::Preview(UninstallPreview {
            would_remove_shared_binaries: Vec::new(),
            would_keep_shared_binaries: Vec::new(),
            would_remove_plugins: Vec::new(),
            has_mcp_entry: false,
            modified_files: Vec::new(),
        });
        let frame = render_frame(&state, 40, 24);
        // The path has no spaces to break on, so `wrap` legitimately
        // hyphenates it across lines (same as any unbreakable token --
        // `wrap_never_drops_a_byte_no_matter_how_long_the_text` already
        // covers that it loses no content). What must never happen here is
        // `pad`'s truncation marker.
        assert!(
            frame.iter().any(|l| l.contains("Remove ai-text-editor")),
            "confirmation question missing entirely: {frame:?}"
        );
        assert!(
            !frame.iter().any(|l| l.contains("...")),
            "the confirmation screen must never show a truncation marker: {frame:?}"
        );
    }

    #[test]
    fn the_done_report_is_never_truncated_however_narrow() {
        let long_binary = "a-very-long-shared-binary-name-that-does-not-fit-in-forty-columns";
        let report = UninstallReport {
            was_installed: true,
            removed_shared_binaries: vec![long_binary.to_string()],
            kept_shared_binaries: Vec::new(),
            removed_plugins: Vec::new(),
            mcp_entry_removed: false,
            modified_files: Vec::new(),
            permissions_removed: Vec::new(),
            opencode_tui_hint_plugin_removed: false,
        };
        let lines = render_done(&report, 40, 20);
        assert!(
            lines.iter().any(|l| l.contains("removed shared binary")),
            "lines were: {lines:?}"
        );
        assert!(
            !lines.iter().any(|l| l.contains("...")),
            "lines were: {lines:?}"
        );
    }
}
