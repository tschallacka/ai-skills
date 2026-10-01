// MODE: DEV
// PACKAGE: PROD
//! A graphical view of the WHOLE post-picker pipeline -- the actual
//! per-skill copy loop (`install_selected_skills` in `main.rs`) and EVERY
//! post-install step after it (worktrees permissions, planning,
//! project-specifics, MCP registration, interactive-shell, editor steering,
//! agent identity, chat interrupts, agent profiles) -- once the
//! wizard/picker have already decided WHAT to install. Replaces plain
//! `println!` lines and a blocking stdin-read `Confirms::ask` with a skill
//! list colored by status, a progress bar, a scrollable log, and a modal
//! Yes/No/All popup for a question -- the same graphical treatment
//! `ui::wizard`/`ui::run_picker` already give every earlier step, so the
//! whole flow stays one continuous screen from the moment the picker's own
//! choices are confirmed to the moment everything is actually done,
//! instead of dropping back to a wall of scrolling plain text partway
//! through (every step used to print directly via `println!`/a
//! non-bridged `Confirms::ask`, which either corrupted this screen's
//! alternate-screen display if it ran while the screen was still open, or
//! -- the shape this actually shipped as for a while -- ran only after the
//! screen had already closed and restored the terminal, which is exactly
//! the "rushed past in plain text" experience this screen exists to
//! replace). `main.rs`'s own standalone `install` CLI subcommand (always
//! headless, never through this screen) still calls the identical
//! `run_remaining_post_install_steps`/`run_worktrees_permission_step`
//! functions, just with a `PlainSink` -- one set of post-install steps,
//! two ways of watching them run.
//!
//! There is no cancellation: the underlying install/permission work has no
//! cancellation plumbing of its own (a half-applied file copy or permission
//! grant cannot simply be abandoned mid-way), so this screen is
//! deliberately read-only until the worker finishes -- scrolling the log,
//! answering a question, and moving keyboard focus among a question's own
//! buttons are the only interactive actions.

use super::buttons::colorize_button;
use super::input::{self, Key};
use super::mascot::{self, ColorMode};
use super::render::BorderSet;
use super::terminal;
use super::text::{pad, pad_display, wrap};
use std::sync::mpsc::{self, Sender};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRunStatus {
    Pending,
    Running,
    Done,
    Skipped,
}

/// What the worker thread reports back to the render loop.
enum Event {
    Status(usize, SkillRunStatus),
    Log(String),
    /// A yes/no/all question -- the reply channel is answered exactly once,
    /// by whichever of a keypress or a mouse click on the popup's own
    /// buttons the render loop sees first.
    Ask(String, Sender<Answer>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Yes,
    No,
    All,
}

/// What `install_selected_skills` (and `run_worktrees_permission_step`) call
/// instead of `println!` directly, so the exact same code runs whether or
/// not a graphical screen is actually driving it -- see `PlainSink` for the
/// non-graphical fallback (a scripted `install` CLI run, or no tty at all).
pub trait Sink {
    fn log(&mut self, line: &str);
    fn status(&mut self, index: usize, status: SkillRunStatus);
}

/// The non-graphical fallback: prints exactly as this pipeline always has,
/// and ignores status updates, since there is no skill list on screen to
/// color.
pub struct PlainSink;

impl Sink for PlainSink {
    fn log(&mut self, line: &str) {
        println!("{line}");
    }
    fn status(&mut self, _index: usize, _status: SkillRunStatus) {}
}

struct ChannelSink {
    tx: Sender<Event>,
}

impl Sink for ChannelSink {
    fn log(&mut self, line: &str) {
        let _ = self.tx.send(Event::Log(line.to_string()));
    }
    fn status(&mut self, index: usize, status: SkillRunStatus) {
        let _ = self.tx.send(Event::Status(index, status));
    }
}

/// A cheap, cloneable handle the worker thread uses to ask a yes/no/all
/// question and block until the render loop answers it -- `Confirms::ask`'s
/// own graphical path in `main.rs`. `Confirms` keeps its existing plain
/// stdin-reading path entirely unchanged when it holds no bridge, so a
/// scripted or non-tty run is completely unaffected by this screen existing
/// at all.
#[derive(Clone)]
pub struct AskBridge(Sender<Event>);

impl AskBridge {
    /// Blocks until the render loop answers. A dropped reply channel (the
    /// render loop exited without answering, which should not normally
    /// happen while the worker is still running) reads as `No`, the same
    /// fail-safe `Confirms::ask`'s own stdin path already uses for a read
    /// error.
    pub fn ask(&self, prompt: &str) -> Answer {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .0
            .send(Event::Ask(prompt.to_string(), reply_tx))
            .is_err()
        {
            return Answer::No;
        }
        reply_rx.recv().unwrap_or(Answer::No)
    }
}

struct SkillRow {
    name: String,
    status: SkillRunStatus,
    /// Whether this skill already had a destination on disk before this run
    /// started -- known up front (the picker/wizard already answered this
    /// for every selected skill), not something the run itself discovers,
    /// so it never changes once a row is built. Drives both the `i`/`u`
    /// marker prefix and the row's own color: yellow for a fresh install,
    /// green for an update, the same colors `render.rs`'s own
    /// "Install"/"Update" button-label distinction already uses this
    /// session for the identical install-vs-update question.
    already_installed: bool,
}

struct State {
    skills: Vec<SkillRow>,
    log: Vec<String>,
    log_scroll: usize,
    question: Option<(String, Sender<Answer>)>,
    /// Which of the modal's three buttons (Yes/No/All, left to right) has
    /// keyboard focus -- reset to 0 ("Yes") whenever a new question
    /// arrives, so answering always starts from the same, most common
    /// choice rather than remembering wherever a previous question's
    /// answer left it.
    modal_focus: usize,
}

impl State {
    fn new(skills: &[(String, bool)]) -> Self {
        State {
            skills: skills
                .iter()
                .map(|(name, already_installed)| SkillRow {
                    name: name.clone(),
                    status: SkillRunStatus::Pending,
                    already_installed: *already_installed,
                })
                .collect(),
            log: Vec::new(),
            log_scroll: 0,
            question: None,
            modal_focus: 0,
        }
    }

    fn progress(&self) -> (usize, usize) {
        let total = self.skills.len().max(1);
        let done = self
            .skills
            .iter()
            .filter(|s| !matches!(s.status, SkillRunStatus::Pending))
            .count();
        (done, total)
    }
}

/// Runs `work` and returns its result. On a real terminal, this is a full
/// graphical screen: `work` runs on a background thread while this function
/// drives the render loop on the calling thread, translating its `Sink`/
/// `AskBridge` calls into skill-list colors, log lines, and a modal popup.
/// With no tty (a script, a CI job, `curl | sh`), `work` just runs directly
/// against a `PlainSink` and no bridge -- the exact plain-text behavior this
/// pipeline always had, since there is no terminal to draw a screen on.
pub fn run<R: Send + 'static>(
    skills: &[(String, bool)],
    work: impl FnOnce(&mut dyn Sink, Option<AskBridge>) -> R + Send + 'static,
) -> R {
    if !terminal::is_tty() {
        let mut sink = PlainSink;
        return work(&mut sink, None);
    }

    let (tx, rx) = mpsc::channel::<Event>();
    let bridge = AskBridge(tx.clone());
    let (result_tx, result_rx) = mpsc::channel::<R>();
    let worker = thread::spawn(move || {
        let mut sink = ChannelSink { tx };
        let result = work(&mut sink, Some(bridge));
        let _ = result_tx.send(result);
    });

    let saved = terminal::enter();
    let color_mode = mascot::detect_color_mode();
    let unicode = mascot::detect_utf8_capable();
    let key_rx = terminal::spawn_reader();
    let mut state = State::new(skills);

    let final_result = loop {
        while let Ok(event) = rx.try_recv() {
            apply_event(&mut state, event);
        }
        let (cols, rows) = terminal::size();
        terminal::draw(&render_frame(&state, cols, rows, color_mode, unicode));
        // The worker's result lands only after every event it sent has
        // already been drained above (it is the last thing the worker
        // thread does), so seeing it here means the screen is fully
        // caught up, not merely idle between two log lines.
        if let Ok(result) = result_rx.try_recv() {
            break result;
        }
        match input::read_key(&key_rx) {
            Key::Tick => continue,
            Key::Eof => continue,
            key => handle_key(&mut state, key, cols),
        }
    };

    terminal::leave(&saved);
    let _ = worker.join();
    final_result
}

fn apply_event(state: &mut State, event: Event) {
    match event {
        Event::Status(i, status) => {
            if let Some(row) = state.skills.get_mut(i) {
                row.status = status;
            }
        }
        Event::Log(line) => state.log.push(line),
        Event::Ask(prompt, reply) => {
            state.question = Some((prompt, reply));
            state.modal_focus = 0;
        }
    }
}

/// The modal's three buttons, left to right -- `state.modal_focus`'s own
/// index into this same order.
fn modal_focus_answer(focus: usize) -> Answer {
    match focus {
        0 => Answer::Yes,
        1 => Answer::No,
        _ => Answer::All,
    }
}

fn handle_key(state: &mut State, key: Key, cols: usize) {
    if state.question.is_some() {
        let answer = match key {
            // The letter shortcuts answer directly regardless of focus --
            // muscle memory from before keyboard focus existed here, kept
            // rather than removed now that arrow/Tab navigation also works.
            Key::Char('y') => Some(Answer::Yes),
            Key::Char('n') => Some(Answer::No),
            Key::Char('a') => Some(Answer::All),
            Key::Escape => Some(Answer::No),
            // Enter activates whichever button keyboard focus is currently
            // on -- the same "focus moves, Enter activates" shape the
            // skill picker's own DETAILS pane buttons already use.
            Key::Enter => Some(modal_focus_answer(state.modal_focus)),
            Key::Left | Key::Char('h') | Key::ShiftTab => {
                state.modal_focus = state.modal_focus.saturating_sub(1);
                None
            }
            Key::Right | Key::Char('l') | Key::Tab => {
                state.modal_focus = (state.modal_focus + 1).min(2);
                None
            }
            Key::Click { col, row } => modal_click_answer(state, cols, col, row),
            _ => None,
        };
        if let Some(answer) = answer {
            if let Some((_, reply)) = state.question.take() {
                let _ = reply.send(answer);
            }
        }
        return;
    }
    match key {
        Key::Up | Key::Char('k') => {
            state.log_scroll = state.log_scroll.saturating_sub(1);
        }
        Key::Down | Key::Char('j') => {
            state.log_scroll += 1;
        }
        _ => {}
    }
}

/// Sized to the longest skill name, the same "content decides the width"
/// rule `layout::list_width` uses for the skill picker's own list pane --
/// shared by `render_frame` and `modal_click_answer` so a click is always
/// tested against exactly the columns the frame actually drew.
fn left_pane_width(state: &State) -> usize {
    state
        .skills
        .iter()
        .map(|s| s.name.chars().count())
        .max()
        .unwrap_or(4)
        .clamp(12, 30)
        + 6
}

/// How many lines of the right pane's own TOP section (see `render_frame`)
/// come before the modal's own `lines[0]` -- the progress bar's own row,
/// then one blank separator row. `modal_click_answer` and `render_frame`
/// both need this to agree on where the modal's own rows actually land
/// once it is no longer the only thing in the pane.
const MODAL_TOP_OFFSET: usize = 2;

/// Which button, if any, a click at `(col, row)` (1-based, exactly what
/// `Key::Click` reports) landed on -- `None` while no question is pending,
/// this function is only ever consulted from `handle_key`'s own
/// `state.question.is_some()` branch.
fn modal_click_answer(state: &State, cols: usize, col: u16, row: u16) -> Option<Answer> {
    let (prompt, _) = state.question.as_ref()?;
    if col == 0 || row == 0 {
        return None;
    }
    let left_w = left_pane_width(state);
    let right_w = cols.saturating_sub(left_w + 3);
    let modal = build_modal(prompt, right_w);
    // title(row 1) + top border(row 2) + the top section's own progress-bar
    // and blank rows + the button's own 0-based row within the modal.
    let button_abs_row = 2 + MODAL_TOP_OFFSET + modal.button_row + 1;
    if row as usize != button_abs_row {
        return None;
    }
    let content_start_col = left_w + 3; // past the left border, pane, and divider
    if (col as usize) < content_start_col {
        return None;
    }
    let content_col = (col as usize) - content_start_col;
    modal
        .buttons
        .iter()
        .find(|(_, start, end)| (*start..*end).contains(&content_col))
        .map(|(answer, _, _)| *answer)
}

fn render_frame(
    state: &State,
    cols: usize,
    rows: usize,
    color_mode: ColorMode,
    unicode: bool,
) -> Vec<String> {
    let b = BorderSet::for_unicode(unicode);
    let (done, total) = state.progress();
    let title = pad(&format!(" Installing -- {done}/{total} skills "), cols);
    let hint = pad(
        " Up/Dn scroll log  Tab/Left/Right focus a question  Enter answer",
        cols,
    );
    let left_w = left_pane_width(state);
    let right_w = cols.saturating_sub(left_w + 3);
    let body_rows = rows.saturating_sub(4).max(1);

    let mut out = Vec::with_capacity(rows);
    out.push(title);
    out.push(format!(
        "{}{}{}{}{}",
        b.corner_tl,
        pad_dash("UPDATING/INSTALLING SKILLS", left_w, b.horizontal),
        b.divider_top,
        pad_dash("PROGRESS", right_w, b.horizontal),
        b.corner_tr
    ));

    let left_lines: Vec<String> = state
        .skills
        .iter()
        .map(|s| colorize_skill_row(s, left_w, color_mode))
        .collect();
    // The log is always visible, in the pane's own bottom half, below
    // whatever is on top (the progress bar, and the question modal while
    // one is pending) -- it used to be replaced outright by a pending
    // question, which hid every line already logged the moment one came
    // up. `top_rows`/`bottom_rows` split `body_rows` evenly; a modal taller
    // than the top half is cut off the same way any other overflowing
    // content here already is (the `.get(i)` fallback below), rather than
    // stealing room from the log.
    let top_rows = body_rows / 2;
    let bottom_rows = body_rows - top_rows;
    let top_lines = top_section_lines(state, right_w, done, total, unicode, color_mode);
    let bottom_lines = bottom_log_lines(state, right_w);
    let mut right_lines = Vec::with_capacity(body_rows);
    for i in 0..top_rows {
        right_lines.push(
            top_lines
                .get(i)
                .cloned()
                .unwrap_or_else(|| pad("", right_w)),
        );
    }
    for i in 0..bottom_rows {
        right_lines.push(
            bottom_lines
                .get(i)
                .cloned()
                .unwrap_or_else(|| pad("", right_w)),
        );
    }

    for i in 0..body_rows {
        let l = left_lines
            .get(i)
            .cloned()
            .unwrap_or_else(|| pad("", left_w));
        let r = right_lines
            .get(i)
            .cloned()
            .unwrap_or_else(|| pad("", right_w));
        out.push(format!("{}{l}{}{r}{}", b.vertical, b.vertical, b.vertical));
    }
    out.push(format!(
        "{}{}{}{}{}",
        b.corner_bl,
        std::iter::repeat_n(b.horizontal, left_w).collect::<String>(),
        b.divider_bottom,
        std::iter::repeat_n(b.horizontal, right_w).collect::<String>(),
        b.corner_br
    ));
    out.push(hint);
    out
}

fn pad_dash(label: &str, width: usize, fill: char) -> String {
    if label.len() >= width {
        return label[..width.min(label.len())].to_string();
    }
    format!(
        "{label}{}",
        std::iter::repeat_n(fill, width - label.len()).collect::<String>()
    )
}

// Colors are by KIND (install vs. update), not by run-status -- see
// `colorize_skill_row`'s own doc comment.
const INSTALL_COLOR: (u8, u8, u8) = (170, 150, 30); // yellow: a fresh install
const UPDATE_COLOR: (u8, u8, u8) = (35, 140, 70); // green: updating something already there
const SKIPPED_COLOR: (u8, u8, u8) = (150, 150, 150); // grey: unconditional, regardless of kind

/// `i`/`u` names what KIND of row this is (a fresh install or an update to
/// something already there), ahead of the existing run-status marker
/// (` `/`>`/`*`/`-`) that names WHERE this particular run is with it --
/// "u * ai-text-editor" reads as "update, done". Shown regardless of
/// status (even Pending), since the install-vs-update distinction is known
/// up front and does not change as the run proceeds.
fn colorize_skill_row(row: &SkillRow, width: usize, mode: ColorMode) -> String {
    let kind_letter = if row.already_installed { 'u' } else { 'i' };
    let marker = match row.status {
        SkillRunStatus::Pending => ' ',
        SkillRunStatus::Running => '>',
        SkillRunStatus::Done => '*',
        SkillRunStatus::Skipped => '-',
    };
    let plain = pad(&format!("{kind_letter} {marker} {}", row.name), width);
    match row.status {
        SkillRunStatus::Pending => plain,
        SkillRunStatus::Skipped => colorize_button(mode, &plain, SKIPPED_COLOR, false),
        // Running and Done share the same install-vs-update coloring --
        // yellow for a fresh install, green for an update -- rather than
        // the OLD scheme (yellow only while running, green only once
        // done): the kind of change a row represents does not change
        // partway through it finishing.
        SkillRunStatus::Running | SkillRunStatus::Done => {
            let bg = if row.already_installed {
                UPDATE_COLOR
            } else {
                INSTALL_COLOR
            };
            colorize_button(mode, &plain, bg, false)
        }
    }
}

fn progress_bar(done: usize, total: usize, width: usize, unicode: bool) -> String {
    let pct = (done * 100).checked_div(total).unwrap_or(0);
    let fill_char = if unicode { '█' } else { '#' };
    let bar_width = width.saturating_sub(7); // "[" + "]" + " NNN%"
    let filled = (bar_width * done).checked_div(total).unwrap_or(0);
    format!(
        "[{}{}] {pct:>3}%",
        std::iter::repeat_n(fill_char, filled).collect::<String>(),
        " ".repeat(bar_width.saturating_sub(filled)),
    )
}

/// The right pane's own TOP section: the progress bar, then the question
/// modal when one is pending -- see `render_frame`'s own doc comment for
/// why this no longer also holds the log (it used to; the log is always in
/// the BOTTOM section now, via `bottom_log_lines`, so it stays visible
/// underneath a pending question instead of being replaced by it).
fn top_section_lines(
    state: &State,
    width: usize,
    done: usize,
    total: usize,
    unicode: bool,
    color_mode: ColorMode,
) -> Vec<String> {
    // `pad` measures by byte length; `progress_bar`'s own `fill_char` can be
    // `█` (3 bytes, 1 display column) when `unicode` is set, which made
    // `pad`'s truncation branch slice mid-character and panic -- reproduced
    // live. `pad_display` measures by character count instead, matching
    // `progress_bar`'s own `bar_width` arithmetic exactly (see `text::
    // pad_display`'s own doc comment).
    let mut lines = vec![pad_display(
        &progress_bar(done, total, width, unicode),
        width,
    )];
    if let Some((prompt, _)) = &state.question {
        lines.push(pad("", width));
        lines.extend(modal_render_lines(
            prompt,
            width,
            color_mode,
            state.modal_focus,
        ));
    }
    lines
}

/// The right pane's own BOTTOM section: the cumulative install/permission
/// log, newest entry first -- "the install output log should be ... under
/// the yes no wizard, and ... cumulative, and newest at top." `log_scroll`
/// skips from the front of this (now reversed) order, so scrolling DOWN
/// still means "go further back in time", the same direction it always
/// meant when the log read oldest-first.
fn bottom_log_lines(state: &State, width: usize) -> Vec<String> {
    let mut lines = vec![pad("LOG", width)];
    let visible_log = state.log.iter().rev().skip(state.log_scroll);
    for line in visible_log {
        for wrapped in wrap(line, width) {
            lines.push(pad(&wrapped, width));
        }
    }
    lines
}

const MODAL_YES_BG: (u8, u8, u8) = (25, 110, 60);
const MODAL_NO_BG: (u8, u8, u8) = (120, 45, 45);
const MODAL_ALL_BG: (u8, u8, u8) = (55, 90, 130);

const YES_LABEL: &str = "[ Yes (y) ]";
const NO_LABEL: &str = "[ No (n) ]";
const ALL_LABEL: &str = "[ All (a) ]";

/// The modal's plain (uncolored) content, plus exactly where its own button
/// row and each button's own column span landed -- built once and read by
/// BOTH `modal_render_lines` (to colorize that one row at actual draw time)
/// and `modal_click_answer` (to test a click against it), so the two can
/// never drift apart the way two independently maintained copies could.
struct ModalLayout {
    lines: Vec<String>,
    button_row: usize,
    /// `(answer, start content-column, end content-column exclusive)` for
    /// each button, in the order drawn.
    buttons: [(Answer, usize, usize); 3],
}

fn build_modal(prompt: &str, width: usize) -> ModalLayout {
    let mut lines = vec![String::new(), "-- QUESTION --".to_string(), String::new()];
    lines.extend(wrap(prompt, width));
    lines.push(String::new());

    let mut row = String::new();
    let push_button = |row: &mut String, label: &str| -> (usize, usize) {
        if !row.is_empty() {
            row.push_str("  ");
        }
        let start = row.chars().count();
        row.push_str(label);
        (start, row.chars().count())
    };
    let (yes_start, yes_end) = push_button(&mut row, YES_LABEL);
    let (no_start, no_end) = push_button(&mut row, NO_LABEL);
    let (all_start, all_end) = push_button(&mut row, ALL_LABEL);

    let button_row = lines.len();
    lines.push(row);
    ModalLayout {
        lines,
        button_row,
        buttons: [
            (Answer::Yes, yes_start, yes_end),
            (Answer::No, no_start, no_end),
            (Answer::All, all_start, all_end),
        ],
    }
}

/// `focus`: the index (into `buttons`, left to right) of whichever button
/// keyboard focus is currently on -- reverse video, the same "this is where
/// input lands" treatment every other keyboard-focused control in this
/// crate already gets, layered on top of (not replacing) each button's own
/// resting background color.
fn colorize_modal_buttons(
    plain_row: &str,
    buttons: &[(Answer, usize, usize); 3],
    width: usize,
    mode: ColorMode,
    focus: usize,
) -> String {
    let chars: Vec<char> = plain_row.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    for (index, (answer, start, end)) in buttons.iter().enumerate() {
        out.extend(chars[i..*start].iter());
        let seg: String = chars[*start..*end].iter().collect();
        let bg = match answer {
            Answer::Yes => MODAL_YES_BG,
            Answer::No => MODAL_NO_BG,
            Answer::All => MODAL_ALL_BG,
        };
        out.push_str(&colorize_button(mode, &seg, bg, index == focus));
        i = *end;
    }
    out.extend(chars[i..].iter());
    // Padded by VISIBLE length, not `pad`'s byte-length math, which would
    // miscount the SGR bytes the loop above just added as display columns.
    out.push_str(&" ".repeat(width.saturating_sub(chars.len())));
    out
}

fn modal_render_lines(prompt: &str, width: usize, mode: ColorMode, focus: usize) -> Vec<String> {
    let modal = build_modal(prompt, width);
    modal
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == modal.button_row {
                colorize_modal_buttons(line, &modal.buttons, width, mode, focus)
            } else {
                pad(line, width)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every name a fresh install (`already_installed: false`) -- the
    /// common case for most tests here, which care about run-status
    /// mechanics, not the install-vs-update distinction specifically.
    fn names(n: &[&str]) -> Vec<(String, bool)> {
        n.iter().map(|s| (s.to_string(), false)).collect()
    }

    #[test]
    fn state_starts_with_every_skill_pending() {
        let state = State::new(&names(&["a", "b"]));
        assert!(state
            .skills
            .iter()
            .all(|s| s.status == SkillRunStatus::Pending));
        assert_eq!(state.progress(), (0, 2));
    }

    #[test]
    fn a_status_event_updates_progress() {
        let mut state = State::new(&names(&["a", "b"]));
        apply_event(&mut state, Event::Status(0, SkillRunStatus::Done));
        assert_eq!(state.skills[0].status, SkillRunStatus::Done);
        assert_eq!(state.progress(), (1, 2));
    }

    #[test]
    fn a_log_event_is_appended() {
        let mut state = State::new(&names(&["a"]));
        apply_event(&mut state, Event::Log("installed a -> /x".to_string()));
        assert_eq!(state.log, vec!["installed a -> /x".to_string()]);
    }

    #[test]
    fn an_ask_event_opens_a_question() {
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        apply_event(&mut state, Event::Ask("proceed?".to_string(), tx));
        assert!(state.question.is_some());
    }

    #[test]
    fn pressing_y_answers_yes_and_clears_the_question() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), tx));
        handle_key(&mut state, Key::Char('y'), 80);
        assert!(state.question.is_none());
        assert_eq!(rx.try_recv(), Ok(Answer::Yes));
    }

    #[test]
    fn pressing_n_answers_no() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), tx));
        handle_key(&mut state, Key::Char('n'), 80);
        assert_eq!(rx.try_recv(), Ok(Answer::No));
    }

    #[test]
    fn pressing_a_answers_all() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), tx));
        handle_key(&mut state, Key::Char('a'), 80);
        assert_eq!(rx.try_recv(), Ok(Answer::All));
    }

    #[test]
    fn a_new_question_starts_keyboard_focus_on_yes() {
        let mut state = State::new(&names(&["a"]));
        state.modal_focus = 2;
        let (tx, _rx) = mpsc::channel();
        apply_event(&mut state, Event::Ask("proceed?".to_string(), tx));
        assert_eq!(state.modal_focus, 0);
    }

    #[test]
    fn right_and_tab_move_modal_focus_forward_left_moves_it_back() {
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), tx));
        handle_key(&mut state, Key::Right, 80);
        assert_eq!(state.modal_focus, 1);
        handle_key(&mut state, Key::Tab, 80);
        assert_eq!(state.modal_focus, 2);
        // Already the last button: no further right to go to.
        handle_key(&mut state, Key::Right, 80);
        assert_eq!(state.modal_focus, 2);
        handle_key(&mut state, Key::Left, 80);
        assert_eq!(state.modal_focus, 1);
    }

    #[test]
    fn enter_activates_whichever_button_has_keyboard_focus() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), tx));
        state.modal_focus = 1; // "No"
        handle_key(&mut state, Key::Enter, 80);
        assert!(state.question.is_none());
        assert_eq!(rx.try_recv(), Ok(Answer::No));
    }

    #[test]
    fn clicking_the_no_button_answers_no() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("Create the directory?".to_string(), tx));
        let cols = 80;
        // Find the exact click coordinates the same way a real click would
        // be tested: recompute the modal layout and pick a column inside
        // the "No" button's own span.
        let left_w = left_pane_width(&state);
        let right_w = cols - (left_w + 3);
        let modal = build_modal("Create the directory?", right_w);
        let (_, no_start, _) = modal.buttons[1];
        let col = (left_w + 3 + no_start + 1) as u16;
        let row = (2 + MODAL_TOP_OFFSET + modal.button_row + 1) as u16;
        handle_key(&mut state, Key::Click { col, row }, cols);
        assert!(state.question.is_none());
        assert_eq!(rx.try_recv(), Ok(Answer::No));
    }

    #[test]
    fn a_click_off_the_button_row_does_not_answer() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("Create the directory?".to_string(), tx));
        handle_key(&mut state, Key::Click { col: 5, row: 3 }, 80);
        assert!(state.question.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn scrolling_the_log_only_works_with_no_pending_question() {
        let mut state = State::new(&names(&["a"]));
        handle_key(&mut state, Key::Down, 80);
        assert_eq!(state.log_scroll, 1);
    }

    #[test]
    fn ask_bridge_reads_as_no_when_the_render_loop_is_gone() {
        let (tx, rx) = mpsc::channel::<Event>();
        drop(rx);
        let bridge = AskBridge(tx);
        assert_eq!(bridge.ask("anything?"), Answer::No);
    }

    #[test]
    fn run_falls_back_to_a_plain_sink_with_no_tty() {
        // The test harness's own stdin is never a real tty, so this
        // exercises the non-graphical branch directly -- no thread, no
        // terminal raw mode, `work` runs synchronously against a
        // `PlainSink` and gets `None` for its bridge.
        let bridge_was_none = run(&names(&["a"]), |_sink, bridge| bridge.is_none());
        assert!(bridge_was_none);
    }

    #[test]
    fn plain_sink_ignores_status_and_only_logs() {
        let mut sink = PlainSink;
        sink.status(0, SkillRunStatus::Done); // must not panic; nothing to assert on stdout here
        sink.log("a line");
    }

    #[test]
    fn progress_bar_is_full_width_capped_and_shows_the_percentage() {
        let bar = progress_bar(1, 2, 30, false);
        assert!(bar.contains("50%"));
        assert_eq!(bar.chars().count(), 30);
    }

    #[test]
    fn progress_bar_handles_zero_total_without_dividing_by_zero() {
        let bar = progress_bar(0, 0, 20, false);
        assert!(bar.contains("0%"));
    }

    #[test]
    fn a_unicode_progress_bar_pads_without_panicking() {
        // Reproduces a real crash: `█` is 3 bytes but 1 display column, and
        // `pad`'s own byte-length truncation sliced mid-character --
        // `thread 'main' panicked ... end byte index 84 is not a char
        // boundary; it is inside '█'`. `top_section_lines` is the actual
        // call site that panicked; `progress_bar` alone (the existing tests
        // above) never reached `pad` at all.
        let state = State::new(&names(&["a", "b", "c"]));
        let lines = top_section_lines(&state, 40, 2, 3, true, ColorMode::None);
        for line in &lines {
            assert_eq!(line.chars().count(), 40, "line was: {line:?}");
        }
    }

    #[test]
    fn colorize_skill_row_marks_each_status_distinctly() {
        let done = SkillRow {
            name: "a".to_string(),
            status: SkillRunStatus::Done,
            already_installed: false,
        };
        let pending = SkillRow {
            name: "b".to_string(),
            status: SkillRunStatus::Pending,
            already_installed: false,
        };
        let done_row = colorize_skill_row(&done, 20, ColorMode::TrueColor);
        let pending_row = colorize_skill_row(&pending, 20, ColorMode::TrueColor);
        assert!(done_row.contains("\x1b["));
        assert!(!pending_row.contains("\x1b["));
        assert!(done_row.contains('a'));
        assert!(pending_row.contains('b'));
    }

    #[test]
    fn colorize_skill_row_marks_install_yellow_and_update_green() {
        let install = SkillRow {
            name: "a".to_string(),
            status: SkillRunStatus::Done,
            already_installed: false,
        };
        let update = SkillRow {
            name: "b".to_string(),
            status: SkillRunStatus::Done,
            already_installed: true,
        };
        let install_row = colorize_skill_row(&install, 20, ColorMode::TrueColor);
        let update_row = colorize_skill_row(&update, 20, ColorMode::TrueColor);
        assert!(install_row.contains("i * a"));
        assert!(update_row.contains("u * b"));
        assert!(install_row.contains(&format!(
            "\x1b[48;2;{};{};{}m",
            INSTALL_COLOR.0, INSTALL_COLOR.1, INSTALL_COLOR.2
        )));
        assert!(update_row.contains(&format!(
            "\x1b[48;2;{};{};{}m",
            UPDATE_COLOR.0, UPDATE_COLOR.1, UPDATE_COLOR.2
        )));
    }

    #[test]
    fn render_frame_every_line_is_exactly_cols_wide_when_uncolored() {
        let state = State::new(&names(&["a", "b"]));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        assert_eq!(frame.len(), 24);
        for line in &frame {
            assert_eq!(line.chars().count(), 80, "line was: {line:?}");
        }
    }

    #[test]
    fn a_pending_question_shows_the_modal_in_the_top_section() {
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("Create the directory?".to_string(), tx));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(joined.contains("QUESTION"));
        assert!(joined.contains("Create the directory?"));
        assert!(joined.contains("Yes (y)"));
        assert!(joined.contains("No (n)"));
        assert!(joined.contains("All (a)"));
    }

    #[test]
    fn the_log_stays_visible_underneath_a_pending_question() {
        // "the install output log should be in the bottom half pane, under
        // the yes no wizard" -- a pending question used to replace the log
        // outright; now it sits in the pane's own top half, with the log
        // still showing below it.
        let mut state = State::new(&names(&["a"]));
        state.log.push("installed a -> /tmp/x".to_string());
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("Create the directory?".to_string(), tx));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Create the directory?"));
        assert!(joined.contains("installed a -> /tmp/x"));
    }

    #[test]
    fn the_log_pane_shows_the_worker_s_log_lines() {
        let mut state = State::new(&names(&["a"]));
        state.log.push("installed a -> /tmp/x".to_string());
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        assert!(frame.iter().any(|l| l.contains("installed a -> /tmp/x")));
    }

    #[test]
    fn the_log_shows_newest_entries_first() {
        let mut state = State::new(&names(&["a"]));
        state.log.push("first".to_string());
        state.log.push("second".to_string());
        let lines = bottom_log_lines(&state, 40);
        let first_idx = lines.iter().position(|l| l.contains("first")).unwrap();
        let second_idx = lines.iter().position(|l| l.contains("second")).unwrap();
        assert!(
            second_idx < first_idx,
            "the newer entry should come first: {lines:?}"
        );
    }

    #[test]
    fn the_skills_header_is_renamed() {
        // A short skill name like "a" clamps the left pane down to its own
        // 18-column floor (`left_pane_width`'s `clamp(12, 30) + 6`), which
        // truncates the full header -- checked as a prefix, the same
        // substring any real, wider pane would also show in full.
        let state = State::new(&names(&["a"]));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        assert!(frame[1].contains("UPDATING/INSTALL"));
    }
}
