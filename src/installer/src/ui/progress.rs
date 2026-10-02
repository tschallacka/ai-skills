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
//!
//! Every question carries its own end-user-facing explanation (`AskBridge::
//! ask`'s `explanation` argument), shown in a scrollable panel under its own
//! buttons rather than relying on the log alone -- an end user asked to
//! grant a permission with no idea what it is for has no basis to answer.
//! That panel sizes itself to whatever the top section actually has free
//! (`top_section_lines`' `available_rows`), rather than a fixed handful of
//! lines that left the rest of a tall terminal sitting empty. The progress
//! bar's own percentage includes one extra unit standing for "every
//! post-install step", not just the file-copy loop, so it does not read
//! 100% while real work (a pending question, a permission grant) is still
//! happening; and the screen itself stays open, showing a large centered
//! "DONE!" in that same now-empty top section, acknowledged with Enter or
//! Escape, rather than tearing down the instant the worker thread returns
//! and dropping the whole run's own summary into a wall of plain text after
//! the fact.
//!
//! The log is a fixed-height tail (`LOG_CONTENT_ROWS`, never resized by
//! whatever else is on screen), oldest entry at the top and newest at the
//! bottom like any ordinary terminal scrollback, with Up/Down paging further
//! back into history and toward the live end. The left pane's skill list
//! gets the same mascot every other screen in this crate shows, drawn
//! underneath it when the terminal is tall enough to leave room.

use super::buttons::colorize_button;
use super::input::{self, Key};
use super::mascot::{self, ColorMode};
use super::render::BorderSet;
use super::terminal;
use super::text::{pad, pad_display, titled_rule, wrap};
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
    /// buttons the render loop sees first. `explanation` is the longer,
    /// end-user-facing "what is this and why does it need my permission"
    /// text, shown in the modal's own scrollable panel underneath the
    /// buttons -- see `AskBridge::ask`'s own doc comment for why a question
    /// carries this with it instead of relying on the user having already
    /// scrolled the log back far enough to find it.
    Ask(String, String, Sender<Answer>),
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
    /// Blocks until the render loop answers. `explanation` is shown in the
    /// modal's own scrollable panel, below its Yes/No/All buttons -- it
    /// exists because a bare one-line question ("Grant ... read/write ...?")
    /// gives an end user with zero context on the skill asking for it no way
    /// to judge whether to say yes, and the log line that used to carry that
    /// context sat buried under whatever the question itself had already
    /// pushed the log pane out of view -- every call site now hands its
    /// explanation to the question that needs it, instead of relying on the
    /// log alone. A dropped reply channel (the render loop exited without
    /// answering, which should not normally happen while the worker is
    /// still running) reads as `No`, the same fail-safe `Confirms::ask`'s
    /// own stdin path already uses for a read error.
    pub fn ask(&self, prompt: &str, explanation: &str) -> Answer {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .0
            .send(Event::Ask(
                prompt.to_string(),
                explanation.to_string(),
                reply_tx,
            ))
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
    /// `(prompt, explanation, reply)` -- see `Event::Ask`'s own doc comment
    /// for what `explanation` is and why it travels with the question
    /// rather than living only in the log.
    question: Option<(String, String, Sender<Answer>)>,
    /// Which of the modal's three buttons (Yes/No/All, left to right) has
    /// keyboard focus -- reset to 0 ("Yes") whenever a new question
    /// arrives, so answering always starts from the same, most common
    /// choice rather than remembering wherever a previous question's
    /// answer left it.
    modal_focus: usize,
    /// How many lines of the pending question's own explanation panel have
    /// scrolled past -- reset to 0 whenever a new question arrives, same as
    /// `modal_focus`. Unlike `log_scroll`, this has no upper clamp either;
    /// see `log_scroll`'s own precedent for why overscrolling past the end
    /// is harmless (it just windows to nothing further) and Up/Down always
    /// being able to walk it back is what matters.
    modal_text_scroll: usize,
    /// Whether the worker thread has finished and handed back its result --
    /// set once `run`'s own loop sees it, so the screen can show a final
    /// "installation complete" state and wait for an acknowledging keypress
    /// instead of tearing the screen down the instant the last background
    /// line is still being read. Purely a render flag; `run` itself tracks
    /// the actual `R` value separately, since this type does not know or
    /// care what `R` is.
    done: bool,
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
            modal_text_scroll: 0,
            done: false,
        }
    }

    /// Per-skill progress only (the file-copy loop) -- what the title bar's
    /// own "N/M skills" text names, since "skills" there should mean skills,
    /// not skills-plus-one-phantom-unit.
    fn skills_progress(&self) -> (usize, usize) {
        let total = self.skills.len().max(1);
        let done = self
            .skills
            .iter()
            .filter(|s| !matches!(s.status, SkillRunStatus::Pending))
            .count();
        (done, total)
    }

    /// Overall run progress, for the percentage bar: every skill's own copy
    /// PLUS one more unit standing for "every post-install step" (worktree
    /// permissions, planning, project-specifics, ...), which only completes
    /// once `run`'s own loop sees the worker thread's result -- "progress
    /// sits at 100%, that's unrealistic, as we're going through questions
    /// that perform actions" was real: every skill file is copied well
    /// before those later steps even start, so a bar driven by the skill
    /// count alone reads as finished while real work is still happening.
    fn progress(&self) -> (usize, usize) {
        let (skills_done, skills_total) = self.skills_progress();
        let total = skills_total + 1;
        let done = skills_done + usize::from(self.done);
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
    let mut pending_result: Option<R> = None;
    let mut eyes = mascot::EyeAnimator::new();

    let final_result = loop {
        while let Ok(event) = rx.try_recv() {
            apply_event(&mut state, event);
        }
        let (cols, rows) = terminal::size();
        terminal::draw(&render_frame(&state, cols, rows, color_mode, unicode));
        // Same mascot every other screen in this crate shows, under the
        // skill list -- centered in the left pane's own width (matching the
        // wizard's centered placement, not flush against the left border),
        // with a blank row above it so it doesn't start flush against the
        // separator line either, and only when the terminal is tall enough
        // to leave it that room below every skill row (the same `mascot_on`
        // gate `render.rs`'s own layout uses, adjusted for the extra blank).
        let body_rows = rows.saturating_sub(4).max(1);
        let left_w = left_pane_width(&state);
        if body_rows >= state.skills.len() + 2 + mascot::HEIGHT {
            let col = 2 + left_w.saturating_sub(mascot::WIDTH) / 2;
            super::draw_mascot_at(
                state.skills.len() + 5,
                col,
                color_mode,
                eyes.current(),
                unicode,
            );
        }
        // The worker's result lands only after every event it sent has
        // already been drained above (it is the last thing the worker
        // thread does), so seeing it here means the screen is fully
        // caught up, not merely idle between two log lines. Rather than
        // breaking the moment it arrives -- which used to tear the graphical
        // screen down and drop straight back to a plain shell prompt, with
        // whatever the worker last logged (the install summary) gone before
        // anyone could read it, "no cool graphical, installation complete at
        // the end" -- the result is held here and `state.done` drives a
        // final "installation complete" frame until the user acknowledges
        // it with Enter or Escape.
        if pending_result.is_none() {
            if let Ok(result) = result_rx.try_recv() {
                pending_result = Some(result);
                state.done = true;
                state
                    .log
                    .push("== Installation complete -- press Enter to continue ==".to_string());
            }
        }
        match input::read_key(&key_rx) {
            Key::Tick => {
                eyes.advance();
                continue;
            }
            Key::Eof => continue,
            Key::Enter | Key::Escape if pending_result.is_some() => {
                break pending_result.take().expect("checked is_some above");
            }
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
        Event::Ask(prompt, explanation, reply) => {
            state.question = Some((prompt, explanation, reply));
            state.modal_focus = 0;
            state.modal_text_scroll = 0;
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
            // 'j'/'k' scroll the question's OWN explanation panel. Up/Down
            // are deliberately NOT used here: this crate's input layer
            // decodes a mouse wheel notch as the exact same `Key::Up`/
            // `Key::Down` a real arrow-key press produces, with no pointer
            // position attached to either -- so dedicating them to the
            // explanation would leave the log unreachable by wheel (or by
            // arrow key) for as long as a question is pending, the same
            // "always scrollable" gap this is fixing, just for the mouse
            // this time. The log keeps Up/Down below, question or not.
            Key::Char('k') => {
                state.modal_text_scroll = state.modal_text_scroll.saturating_sub(1);
                None
            }
            Key::Char('j') => {
                state.modal_text_scroll += 1;
                None
            }
            Key::Up => {
                state.log_scroll += 1;
                None
            }
            Key::Down => {
                state.log_scroll = state.log_scroll.saturating_sub(1);
                None
            }
            Key::Click { col, row } => modal_click_answer(state, cols, col, row),
            _ => None,
        };
        if let Some(answer) = answer {
            if let Some((_, _, reply)) = state.question.take() {
                let _ = reply.send(answer);
            }
        }
        return;
    }
    match key {
        // Same oldest-to-newest, newest-at-the-bottom direction
        // `bottom_log_lines` reads in: Up/`k` (further back in time) grows
        // how far the view has scrolled from the live end, Down/`j` (toward
        // now) shrinks it back. `j`/`k` are accepted here too (nothing else
        // claims them while no question is pending) purely as a vim-style
        // convenience; Up/Down (and so the mouse wheel) are what the log
        // can always rely on, question pending or not. `log_scroll` itself
        // is never clamped here; `bottom_log_lines` clamps it against the
        // actual log length at render time, the same "store unclamped,
        // clamp on display" shape `modal_text_scroll` already uses.
        Key::Up | Key::Char('k') => {
            state.log_scroll += 1;
        }
        Key::Down | Key::Char('j') => {
            state.log_scroll = state.log_scroll.saturating_sub(1);
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

/// Blank columns between the right pane's borders and its content, on each
/// side, so nothing in it sits flush against a border.
const RIGHT_MARGIN: usize = 1;

/// The right pane's content width once `RIGHT_MARGIN` is taken off both sides.
fn right_content_width(right_w: usize) -> usize {
    right_w.saturating_sub(2 * RIGHT_MARGIN)
}

/// Which button, if any, a click at `(col, row)` (1-based, exactly what
/// `Key::Click` reports) landed on -- `None` while no question is pending,
/// this function is only ever consulted from `handle_key`'s own
/// `state.question.is_some()` branch.
fn modal_click_answer(state: &State, cols: usize, col: u16, row: u16) -> Option<Answer> {
    let (prompt, explanation, _) = state.question.as_ref()?;
    if col == 0 || row == 0 {
        return None;
    }
    let left_w = left_pane_width(state);
    let width = right_content_width(cols.saturating_sub(left_w + 3));
    // The explanation panel's own row count never affects the button row's
    // position (it comes after the button row), so the exact value passed
    // here doesn't matter for this click-hotspot math -- `EXPLANATION_
    // VISIBLE_LINES` is as good as any other.
    let modal = build_modal(
        prompt,
        explanation,
        width,
        state.modal_text_scroll,
        EXPLANATION_VISIBLE_LINES,
        '-',
    );
    // title(row 1) + top border(row 2) + the top section's own progress-bar
    // and blank rows + the button's own 0-based row within the modal.
    let button_abs_row = 2 + MODAL_TOP_OFFSET + modal.button_row + 1;
    if row as usize != button_abs_row {
        return None;
    }
    // Past the left border, pane, divider, and the right pane's own margin.
    let content_start_col = left_w + 3 + RIGHT_MARGIN;
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
    let (skill_done, skill_total) = state.skills_progress();
    let title = pad(
        &format!(" Installing -- {skill_done}/{skill_total} skills "),
        cols,
    );
    let hint = pad(
        if state.done {
            " Installation complete -- Enter/Esc to finish  Up/Dn/wheel/j/k scroll log"
        } else if state.question.is_some() {
            " Up/Dn or wheel scroll log  j/k explanation  Tab/Left/Right focus  Enter answer"
        } else {
            " Up/Dn/wheel/j/k scroll log  Tab/Left/Right focus a question  Enter answer"
        },
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
        titled_rule("UPDATING/INSTALLING SKILLS", left_w, b.horizontal, false),
        b.divider_top,
        titled_rule("PROGRESS", right_w, b.horizontal, false),
        b.corner_tr
    ));

    let left_lines: Vec<String> = state
        .skills
        .iter()
        .map(|s| colorize_skill_row(s, left_w, color_mode))
        .collect();
    // The log is always visible, below whatever is on top (the progress
    // bar, and the question modal while one is pending or the "DONE!"
    // banner once everything is) -- it used to be replaced outright by a
    // pending question, which hid every line already logged the moment one
    // came up. Its own section is a FIXED height (`LOG_SECTION_ROWS`,
    // header plus `LOG_CONTENT_ROWS` lines) regardless of what the top
    // section is showing -- "the log staying this size through all
    // windows": a proportional split (half the pane, or whatever a pending
    // question left over) made the log grow and shrink as questions came
    // and went, and made the common case (no question pending, the top
    // section needing only one line for its progress bar) waste the rest of
    // the pane as blank space instead of giving it to either section. The
    // top section gets everything else, so that space is never simply
    // unused; a modal or banner taller than what's left is cut off the same
    // way any other overflowing content here already is (the `.get(i)`
    // fallback below).
    let inner_w = right_content_width(right_w);
    let log_rows = log_content_rows(state, inner_w, body_rows);
    let bottom_rows = (LOG_HEADER_ROWS + log_rows).min(body_rows);
    let top_rows = body_rows - bottom_rows;
    let top_lines = top_section_lines(state, inner_w, done, total, unicode, color_mode, top_rows);
    let bottom_lines = bottom_log_lines(state, inner_w, b.horizontal, log_rows);
    let margin = " ".repeat(RIGHT_MARGIN);
    let framed = |line: Option<&String>| -> String {
        let content = line.cloned().unwrap_or_else(|| pad("", inner_w));
        format!("{margin}{content}{margin}")
    };
    let mut right_lines = Vec::with_capacity(body_rows);
    for i in 0..top_rows {
        right_lines.push(framed(top_lines.get(i)));
    }
    for i in 0..bottom_rows {
        right_lines.push(framed(bottom_lines.get(i)));
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

/// Each letter of the "DONE!" banner, 5 rows tall, plain `#`/space ASCII --
/// no unicode glyph, the same portability floor the progress bar's own
/// non-unicode fallback and `mascot.rs`'s own deliberate ASCII-only choice
/// already hold to, since this banner has no capability probe of its own to
/// gate a fancier version behind.
const DONE_GLYPH_D: [&str; 5] = ["##### ", "#    #", "#    #", "#    #", "##### "];
const DONE_GLYPH_O: [&str; 5] = [" #### ", "#    #", "#    #", "#    #", " #### "];
const DONE_GLYPH_N: [&str; 5] = ["#    #", "##   #", "# #  #", "#  # #", "#   ##"];
const DONE_GLYPH_E: [&str; 5] = ["######", "#     ", "##### ", "#     ", "######"];
const DONE_GLYPH_BANG: [&str; 3] = [" # ", " # ", " # "];

/// The big block-letter "DONE!" banner shown centered in the top section
/// once the whole run has finished -- "when the installation is complete,
/// I'd like that to be displayed in the center in large friendly letters in
/// the top block, that is now Empty. A simple large DONE!" Falls back to a
/// single plain centered line when `width`/`available_rows` can't fit the
/// full block art (a narrow or very short terminal), rather than truncating
/// the art into something unreadable.
/// The mascot's own yellow (`fdc100`), so the banner reads as the same art.
const DONE_YELLOW: (u8, u8, u8) = (0xfd, 0xc1, 0x00);

/// The DONE! art as solid blocks in the mascot's style: each `#` of
/// `done_banner_art` is one pixel, two columns wide like a mascot pixel when
/// it fits, one when narrower, and a plain "DONE!" when even that does not.
fn done_banner_lines(
    width: usize,
    available_rows: usize,
    mode: ColorMode,
    unicode: bool,
) -> Vec<String> {
    let art = done_banner_art(width, available_rows);
    if art.len() == 1 {
        return art;
    }
    for scale in [2, 1] {
        let rows: Vec<String> = art
            .iter()
            .map(|r| {
                r.chars()
                    .flat_map(|c| std::iter::repeat_n(c, scale))
                    .collect()
            })
            .collect();
        if rows[0].chars().count() <= width {
            return rows
                .iter()
                .map(|r| paint_banner_row(&pad(&center_text(r, width), width), mode, unicode))
                .collect();
        }
    }
    vec![pad(&center_text("DONE!", width), width)]
}

/// One centred, padded banner row with its `#` pixels drawn as solid
/// yellow blocks; spaces stay blank, so the visible width is unchanged.
fn paint_banner_row(plain: &str, mode: ColorMode, unicode: bool) -> String {
    let glyph = if unicode { '\u{2588}' } else { '#' };
    let pixels: String = plain
        .chars()
        .map(|c| if c == '#' { glyph } else { c })
        .collect();
    let color = mascot::fg_sgr(mode, DONE_YELLOW);
    if color.is_empty() {
        pixels
    } else {
        format!("{color}{pixels}\x1b[0m")
    }
}

fn done_banner_art(width: usize, available_rows: usize) -> Vec<String> {
    let mut rows = [
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    for (i, row) in rows.iter_mut().enumerate() {
        row.push_str(DONE_GLYPH_D[i]);
        row.push_str("  ");
        row.push_str(DONE_GLYPH_O[i]);
        row.push_str("  ");
        row.push_str(DONE_GLYPH_N[i]);
        row.push_str("  ");
        row.push_str(DONE_GLYPH_E[i]);
        row.push_str("  ");
        // "!" is only 3 rows tall (no descender/dot distinction needed);
        // rows 3 (the gap before a dot) and 4 fall back to blank/dot below.
        row.push_str(if i < 3 {
            DONE_GLYPH_BANG[i]
        } else if i == 4 {
            " # "
        } else {
            "   "
        });
    }
    let banner_width = rows[0].chars().count();
    if available_rows < rows.len() || banner_width > width {
        return vec![pad(&center_text("DONE!", width), width)];
    }
    Vec::from(rows)
}

/// Centers `text` within `width` columns by left-padding with spaces --
/// `text` here is always plain ASCII (the banner's own `#`/space art, or the
/// plain-fallback word itself), so byte length and character count agree
/// and `pad`'s own byte-length measurement is safe to layer on top of this.
fn center_text(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        return text.chars().take(width).collect();
    }
    format!("{}{text}", " ".repeat((width - len) / 2))
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

/// The right pane's own TOP section: the progress bar, then either the
/// question modal (while one is pending) or the "DONE!" banner (once the
/// whole run is) -- see `render_frame`'s own doc comment for why this no
/// longer also holds the log (it used to; the log is always in the BOTTOM
/// section now, via `bottom_log_lines`, so it stays visible underneath a
/// pending question instead of being replaced by it). `available_rows` is
/// the TOTAL row budget `render_frame` actually has for this whole section
/// (progress bar included); the modal's own explanation panel and the DONE
/// banner both size themselves to fill whatever of that is left over,
/// rather than a fixed handful of lines that wasted a tall terminal's
/// genuinely spare space.
fn top_section_lines(
    state: &State,
    width: usize,
    done: usize,
    total: usize,
    unicode: bool,
    color_mode: ColorMode,
    available_rows: usize,
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
    if state.done {
        lines.push(pad("", width));
        let remaining = available_rows.saturating_sub(lines.len());
        let banner = done_banner_lines(width, remaining, color_mode, unicode);
        let top_pad = remaining.saturating_sub(banner.len()) / 2;
        for _ in 0..top_pad {
            lines.push(pad("", width));
        }
        lines.extend(banner);
        return lines;
    }
    if let Some((prompt, explanation, _)) = &state.question {
        lines.push(pad("", width));
        let explanation_rows = available_rows
            .saturating_sub(lines.len())
            .saturating_sub(modal_overhead_lines(prompt, width))
            .max(2);
        lines.extend(modal_render_lines(
            prompt,
            explanation,
            width,
            ModalLook {
                mode: color_mode,
                fill: BorderSet::for_unicode(unicode).horizontal,
            },
            state.modal_focus,
            state.modal_text_scroll,
            explanation_rows,
        ));
    }
    lines
}

/// How many content rows (not counting its own header) the log section
/// always shows -- "the log staying this size through all windows": a
/// fixed budget, not a leftover share of whatever the top section didn't
/// use, so the log pane never grows or shrinks as a question comes and
/// goes or the run finishes. "more log line space can be used there, 8
/// lines".
const LOG_CONTENT_ROWS: usize = 8;
/// The log section's rows above its content: blank, separator, blank.
const LOG_HEADER_ROWS: usize = 3;
/// The fewest log lines shown when a pending question needs the room.
const LOG_MIN_CONTENT_ROWS: usize = 3;

/// The log keeps `LOG_CONTENT_ROWS` unless a pending question would then
/// show fewer than two lines of its explanation; only then does it shrink.
fn log_content_rows(state: &State, width: usize, body_rows: usize) -> usize {
    let Some((prompt, _, _)) = &state.question else {
        return LOG_CONTENT_ROWS;
    };
    let needed_top = MODAL_TOP_OFFSET + modal_overhead_lines(prompt, width) + 2;
    body_rows
        .saturating_sub(needed_top + LOG_HEADER_ROWS)
        .clamp(LOG_MIN_CONTENT_ROWS, LOG_CONTENT_ROWS)
}

/// The right pane's own BOTTOM section: the cumulative install/permission
/// log, in the order it actually happened -- oldest at the top, newest at
/// the bottom, the same direction any ordinary terminal's scrollback reads
/// in. Always exactly `LOG_SECTION_ROWS` tall, windowed as a tail:
/// `log_scroll` is how many lines the view has scrolled back from the live
/// end, clamped here (not in `handle_key`) against the log's actual current
/// length, the same "store unclamped, clamp at render time" shape
/// `modal_text_scroll` already uses. The header is a full-width dashed
/// rule, the same `pad_dash` treatment every other section's own header
/// already gets, rather than a plain padded label -- "ensure the log and
/// this part have separate segments": a bare word gave the log no visible
/// boundary from whatever sits above it.
fn bottom_log_lines(state: &State, width: usize, fill: char, content_rows: usize) -> Vec<String> {
    let mut lines = vec![
        pad("", width),
        titled_rule("LOG", width, fill, false),
        pad("", width),
    ];
    let wrapped: Vec<String> = state
        .log
        .iter()
        .flat_map(|entry| entry.split('\n'))
        .map(printable_log_line)
        .flat_map(|l| wrap(&l, width))
        .collect();
    let total = wrapped.len();
    let max_scroll = total.saturating_sub(content_rows);
    let scroll = state.log_scroll.min(max_scroll);
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(content_rows);
    for line in &wrapped[start..end] {
        lines.push(pad(line, width));
    }
    while lines.len() < LOG_HEADER_ROWS + content_rows {
        lines.push(pad("", width));
    }
    lines
}

/// One physical log line with no control characters left in it. A newline
/// inside a frame row moves the terminal's cursor, so the rest of the frame
/// lands a row lower and the screen scrolls.
fn printable_log_line(line: &str) -> String {
    line.trim_end_matches('\r')
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
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

/// The default explanation-panel height used wherever the caller isn't
/// sizing it against a real screen (`modal_click_answer`, most tests) --
/// `top_section_lines` instead sizes it dynamically, against whatever the
/// actual top section has free, via `modal_overhead_lines`.
const EXPLANATION_VISIBLE_LINES: usize = 6;

/// Wraps `explanation` to `width`, treating each `\n`-separated piece as its
/// own paragraph with a blank separator line between -- `wrap` itself has no
/// notion of a paragraph break (it only ever breaks on spaces), so an
/// explanation built from several distinct points (one per `\n`, the same
/// shape `sink.log` already used for this content one call per line) would
/// otherwise run together into one wall of text with no structure at all.
fn wrap_explanation(explanation: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, paragraph) in explanation.split('\n').enumerate() {
        if i > 0 {
            lines.push(String::new());
        }
        if !paragraph.is_empty() {
            lines.extend(wrap(paragraph, width));
        }
    }
    lines
}

/// How many of the modal's own lines come before its explanation panel's
/// content even starts (the QUESTION header, the wrapped prompt, the button
/// row, and the explanation panel's own header) -- what `top_section_lines`
/// subtracts from its real row budget to find how many rows the explanation
/// itself may actually use. Must stay in exact lockstep with `build_modal`'s
/// own construction order below; a comment there points back here.
fn modal_overhead_lines(prompt: &str, width: usize) -> usize {
    // blank, QUESTION separator, blank, <wrapped prompt>, blank, <buttons>,
    // blank, WHY THIS IS ASKED separator, blank -- then the explanation.
    wrap(prompt, width).len() + 8
}

fn build_modal(
    prompt: &str,
    explanation: &str,
    width: usize,
    text_scroll: usize,
    explanation_rows: usize,
    fill: char,
) -> ModalLayout {
    let mut lines = vec![
        String::new(),
        titled_rule("QUESTION", width, fill, false),
        String::new(),
    ];
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

    // The explanation panel: its own clearly bordered segment below the
    // buttons -- "these questions should have under the buttons a larger,
    // scrollable text area where it is all explained, with the END user who
    // has 0 context awareness why it's needed in mind". Always shown, even
    // for an empty explanation (the windowed loop below just pads out
    // blank), so every question gets the same segment rather than some
    // having one and others not. `modal_overhead_lines` counts every line
    // pushed above this point plus this header -- keep the two in sync.
    lines.push(String::new());
    lines.push(titled_rule(
        "WHY THIS IS ASKED -- j/k to scroll",
        width,
        fill,
        false,
    ));
    lines.push(String::new());
    let wrapped = wrap_explanation(explanation, width);
    // Clamped against how much is actually left to reveal, not just the
    // explanation's own total length -- "if text is all on screen,
    // scrolling shouldn't be possible. Useless to scroll stuff off screen."
    // Without this, scrolling past the point everything already fit
    // visibly pushed the whole, already-fully-shown explanation up out of
    // view instead of refusing to move.
    let max_scroll = wrapped.len().saturating_sub(explanation_rows);
    let start = text_scroll.min(max_scroll);
    for i in 0..explanation_rows {
        lines.push(wrapped.get(start + i).cloned().unwrap_or_default());
    }

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

/// How the modal is drawn: its button colors and its separators' line.
#[derive(Clone, Copy)]
struct ModalLook {
    mode: ColorMode,
    fill: char,
}

fn modal_render_lines(
    prompt: &str,
    explanation: &str,
    width: usize,
    look: ModalLook,
    focus: usize,
    text_scroll: usize,
    explanation_rows: usize,
) -> Vec<String> {
    let ModalLook { mode, fill } = look;
    let modal = build_modal(
        prompt,
        explanation,
        width,
        text_scroll,
        explanation_rows,
        fill,
    );
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
        assert_eq!(state.skills_progress(), (0, 2));
        // +1: the post-install-steps unit, not yet done either.
        assert_eq!(state.progress(), (0, 3));
    }

    #[test]
    fn a_status_event_updates_progress() {
        let mut state = State::new(&names(&["a", "b"]));
        apply_event(&mut state, Event::Status(0, SkillRunStatus::Done));
        assert_eq!(state.skills[0].status, SkillRunStatus::Done);
        assert_eq!(state.skills_progress(), (1, 2));
        assert_eq!(state.progress(), (1, 3));
    }

    #[test]
    fn overall_progress_only_reaches_full_once_the_run_is_marked_done() {
        // "progress sits at 100%, that's unrealistic, as we're going
        // through questions that perform actions" -- every skill finishing
        // its own copy must not alone read as the whole run being done,
        // since the post-install steps (worktrees, planning, ...) still run
        // afterward, in the same screen session.
        let mut state = State::new(&names(&["a", "b"]));
        apply_event(&mut state, Event::Status(0, SkillRunStatus::Done));
        apply_event(&mut state, Event::Status(1, SkillRunStatus::Done));
        let (done, total) = state.progress();
        assert!(done < total, "should not read 100% yet: {done}/{total}");
        state.done = true;
        assert_eq!(state.progress(), (3, 3));
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
        apply_event(
            &mut state,
            Event::Ask("proceed?".to_string(), String::new(), tx),
        );
        assert!(state.question.is_some());
    }

    #[test]
    fn pressing_y_answers_yes_and_clears_the_question() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), String::new(), tx));
        handle_key(&mut state, Key::Char('y'), 80);
        assert!(state.question.is_none());
        assert_eq!(rx.try_recv(), Ok(Answer::Yes));
    }

    #[test]
    fn pressing_n_answers_no() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), String::new(), tx));
        handle_key(&mut state, Key::Char('n'), 80);
        assert_eq!(rx.try_recv(), Ok(Answer::No));
    }

    #[test]
    fn pressing_a_answers_all() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), String::new(), tx));
        handle_key(&mut state, Key::Char('a'), 80);
        assert_eq!(rx.try_recv(), Ok(Answer::All));
    }

    #[test]
    fn a_new_question_starts_keyboard_focus_on_yes() {
        let mut state = State::new(&names(&["a"]));
        state.modal_focus = 2;
        let (tx, _rx) = mpsc::channel();
        apply_event(
            &mut state,
            Event::Ask("proceed?".to_string(), String::new(), tx),
        );
        assert_eq!(state.modal_focus, 0);
    }

    #[test]
    fn a_new_question_also_resets_the_explanation_scroll() {
        let mut state = State::new(&names(&["a"]));
        state.modal_text_scroll = 5;
        let (tx, _rx) = mpsc::channel();
        apply_event(
            &mut state,
            Event::Ask("proceed?".to_string(), String::new(), tx),
        );
        assert_eq!(state.modal_text_scroll, 0);
    }

    #[test]
    fn right_and_tab_move_modal_focus_forward_left_moves_it_back() {
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), String::new(), tx));
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
    fn j_and_k_scroll_the_explanation_panel_while_a_question_is_pending() {
        // 'j'/'k' are the explanation panel's own dedicated keys -- Up/Down
        // (and so the mouse wheel, which decodes to the exact same keys
        // with no pointer position attached) are reserved for the log
        // instead, so the log stays reachable by wheel even while a
        // question is pending; see `up_and_down_scroll_the_log_even_while_
        // a_question_is_pending` for that half.
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), String::new(), tx));
        handle_key(&mut state, Key::Char('j'), 80);
        assert_eq!(state.modal_text_scroll, 1);
        handle_key(&mut state, Key::Char('j'), 80);
        assert_eq!(state.modal_text_scroll, 2);
        handle_key(&mut state, Key::Char('k'), 80);
        assert_eq!(state.modal_text_scroll, 1);
        // It must not have touched the (unrelated, still-zero) log scroll.
        assert_eq!(state.log_scroll, 0);
    }

    #[test]
    fn up_and_down_scroll_the_log_even_while_a_question_is_pending() {
        // "scrollwheel should scroll log up and down, arrows too" -- the
        // mouse wheel decodes as plain `Key::Up`/`Key::Down` with no pointer
        // position attached (see `ui::input`'s own doc comment), so Up/Down
        // must reach the log even while a question is pending, or the wheel
        // (which can never produce 'j'/'k') would have no way to scroll it
        // at all for as long as a question is showing.
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), String::new(), tx));
        handle_key(&mut state, Key::Up, 80);
        assert_eq!(state.log_scroll, 1);
        handle_key(&mut state, Key::Down, 80);
        assert_eq!(state.log_scroll, 0);
        // It must not have touched the (unrelated, still-zero) explanation
        // scroll, and the question must still be pending (Up/Down must not
        // have been mistaken for an answer).
        assert_eq!(state.modal_text_scroll, 0);
        assert!(state.question.is_some());
    }

    #[test]
    fn enter_activates_whichever_button_has_keyboard_focus() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("proceed?".to_string(), String::new(), tx));
        state.modal_focus = 1; // "No"
        handle_key(&mut state, Key::Enter, 80);
        assert!(state.question.is_none());
        assert_eq!(rx.try_recv(), Ok(Answer::No));
    }

    #[test]
    fn clicking_the_no_button_answers_no() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("Create the directory?".to_string(), String::new(), tx));
        let (cols, rows) = (80, 24);
        // Clicked where the frame actually draws it, not where the layout
        // math says it should be, so a moved button cannot pass unnoticed.
        let frame = render_frame(&state, cols, rows, ColorMode::TrueColor, true);
        let (row, col) = frame
            .iter()
            .enumerate()
            .find_map(|(r, line)| {
                let plain = strip_sgr(line);
                let at = plain.find("[ No (n) ]")?;
                Some((r + 1, plain[..at].chars().count() + 4))
            })
            .expect("the No button is on screen");
        handle_key(
            &mut state,
            Key::Click {
                col: col as u16,
                row: row as u16,
            },
            cols,
        );
        assert!(state.question.is_none());
        assert_eq!(rx.try_recv(), Ok(Answer::No));
    }

    /// `row` with its SGR escapes removed: what the terminal shows.
    fn strip_sgr(row: &str) -> String {
        let mut out = String::new();
        let mut chars = row.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn a_click_off_the_button_row_does_not_answer() {
        let mut state = State::new(&names(&["a"]));
        let (tx, rx) = mpsc::channel();
        state.question = Some(("Create the directory?".to_string(), String::new(), tx));
        handle_key(&mut state, Key::Click { col: 5, row: 3 }, 80);
        assert!(state.question.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn scrolling_the_log_only_works_with_no_pending_question() {
        let mut state = State::new(&names(&["a"]));
        handle_key(&mut state, Key::Up, 80);
        assert_eq!(state.log_scroll, 1);
    }

    #[test]
    fn ask_bridge_reads_as_no_when_the_render_loop_is_gone() {
        let (tx, rx) = mpsc::channel::<Event>();
        drop(rx);
        let bridge = AskBridge(tx);
        assert_eq!(bridge.ask("anything?", "why"), Answer::No);
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
        let lines = top_section_lines(&state, 40, 2, 3, true, ColorMode::None, 20);
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
    fn render_frame_shows_a_completion_hint_once_done() {
        // "no cool graphical, installation complete at the end" -- `run`'s
        // own loop sets this once the worker thread hands back its result,
        // and holds the screen open (rather than tearing it straight down to
        // a plain shell prompt) until the user acknowledges it.
        let mut state = State::new(&names(&["a"]));
        state.done = true;
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Installation complete"));
    }

    #[test]
    fn a_done_run_shows_the_big_banner_in_the_now_free_top_section() {
        // "when the installation is complete, I'd like that to be displayed
        // in the center in large friendly letters in the top block, that is
        // now Empty. A simple large DONE!"
        let mut state = State::new(&names(&["a"]));
        state.done = true;
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        // The block-art "D" and "O" glyphs' own top rows.
        assert!(joined.contains("##### "));
        assert!(joined.contains(" #### "));
    }

    #[test]
    fn the_done_banner_falls_back_to_a_plain_line_when_too_narrow() {
        let lines = done_banner_lines(10, 10, ColorMode::None, false);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("DONE!"));
    }

    #[test]
    fn the_done_banner_is_centered_within_its_width() {
        let lines = done_banner_lines(60, 10, ColorMode::None, false);
        assert_eq!(lines.len(), 5);
        for line in &lines {
            assert_eq!(line.chars().count(), 60);
        }
        // The art itself is narrower than 60, so every line should carry
        // leading padding -- it never starts at column 0.
        assert!(lines[0].starts_with(' '));
    }

    #[test]
    fn a_pending_question_shows_the_modal_in_the_top_section() {
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("Create the directory?".to_string(), String::new(), tx));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(joined.contains("QUESTION"));
        assert!(joined.contains("Create the directory?"));
        assert!(joined.contains("Yes (y)"));
        assert!(joined.contains("No (n)"));
        assert!(joined.contains("All (a)"));
    }

    #[test]
    fn the_modal_shows_its_own_explanation_panel() {
        // "these questions should have under the buttons a larger,
        // scrollable text area where it is all explained, with the END user
        // who has 0 context awareness why it's needed in mind".
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some((
            "Grant access?".to_string(),
            "A worktree is a second checked-out copy of this repository.".to_string(),
            tx,
        ));
        let frame = render_frame(&state, 160, 60, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(joined.contains("WHY THIS IS ASKED"));
        assert!(joined.contains("A worktree is a second checked-out copy"));
    }

    #[test]
    fn the_explanation_is_still_visible_on_an_ordinary_sized_terminal() {
        // A straight 50/50 top/bottom split (the log's own original share)
        // routinely truncated the explanation panel before a single word of
        // it reached the screen on a perfectly ordinary 80x24 terminal --
        // the whole point of the panel, silently defeated one layer down
        // from where it looked fixed. `render_frame` now gives the modal
        // most of the room while a question is pending instead.
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some((
            "Grant access?".to_string(),
            "A worktree is a second checked-out copy of this repository.".to_string(),
            tx,
        ));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(joined.contains("WHY THIS IS ASKED"));
        assert!(
            joined.contains("A worktree is a second checked-out"),
            "explanation text was truncated off-screen: {joined}"
        );
    }

    #[test]
    fn the_log_stays_visible_even_with_a_long_explanation_pending() {
        let mut state = State::new(&names(&["a"]));
        state.log.push("installed a -> /tmp/x".to_string());
        let (tx, _rx) = mpsc::channel();
        state.question = Some((
            "Grant access?".to_string(),
            "line one\nline two\nline three\nline four\nline five".to_string(),
            tx,
        ));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(
            joined.contains("installed a -> /tmp/x"),
            "the log was pushed fully off-screen: {joined}"
        );
    }

    #[test]
    fn the_explanation_panel_grows_to_fill_a_tall_terminal_instead_of_wasting_the_space() {
        // "available space is not used" -- a fixed 6-line window left a big
        // blank gap on anything taller than the bare minimum; the panel now
        // sizes itself to whatever `render_frame` actually has free.
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        let long_explanation = (1..=30)
            .map(|i| format!("point number {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        state.question = Some(("Grant access?".to_string(), long_explanation, tx));
        let short_frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let tall_frame = render_frame(&state, 80, 50, ColorMode::None, false);
        let short_count = short_frame
            .iter()
            .filter(|l| l.contains("point number"))
            .count();
        let tall_count = tall_frame
            .iter()
            .filter(|l| l.contains("point number"))
            .count();
        assert!(
            tall_count > short_count,
            "a taller terminal should show more of the explanation: {short_count} vs {tall_count}"
        );
    }

    #[test]
    fn the_explanation_panel_is_windowed_by_its_own_scroll() {
        let explanation = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten";
        let unscrolled = build_modal("q?", explanation, 40, 0, EXPLANATION_VISIBLE_LINES, '-');
        let scrolled = build_modal("q?", explanation, 40, 2, EXPLANATION_VISIBLE_LINES, '-');
        let unscrolled_text = unscrolled.lines.join("\n");
        let scrolled_text = scrolled.lines.join("\n");
        assert!(unscrolled_text.contains("one"));
        // Two lines ("one", a blank separator) scrolled past.
        assert!(!scrolled_text.contains("one"));
        assert!(scrolled_text.contains("two"));
    }

    #[test]
    fn scrolling_is_a_no_op_once_the_whole_explanation_already_fits() {
        // "if text is all on screen, scrolling shouldn't be possible.
        // Useless to scroll stuff off screen." A short explanation that
        // already fits inside `explanation_rows` must not be pushed up out
        // of view by any amount of scrolling.
        let explanation = "one\ntwo";
        let still = build_modal("q?", explanation, 40, 0, EXPLANATION_VISIBLE_LINES, '-');
        let over_scrolled = build_modal("q?", explanation, 40, 50, EXPLANATION_VISIBLE_LINES, '-');
        assert_eq!(still.lines, over_scrolled.lines);
        assert!(over_scrolled.lines.iter().any(|l| l.contains("one")));
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
        state.question = Some(("Create the directory?".to_string(), String::new(), tx));
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        let joined = frame.join("\n");
        assert!(joined.contains("Create the directory?"));
        assert!(joined.contains("installed a -> /tmp/x"));
    }

    #[test]
    fn the_log_header_is_a_dashed_rule_separating_it_from_the_section_above() {
        // The log's separator is the same titled rule as every other
        // section's, with a blank line between it and what is above and
        // below, and the title set in from the edge rather than flush.
        let state = State::new(&names(&["a"]));
        let lines = bottom_log_lines(&state, 40, '-', LOG_CONTENT_ROWS);
        assert_eq!(lines[0].trim(), "");
        assert_eq!(lines[1], format!("--LOG{}", "-".repeat(35)));
        assert_eq!(lines[2].trim(), "");
    }

    #[test]
    fn the_done_banner_is_solid_yellow_blocks_like_the_mascot() {
        let lines = done_banner_lines(100, 10, ColorMode::TrueColor, true);
        assert_eq!(lines.len(), 5);
        let yellow = mascot::fg_sgr(ColorMode::TrueColor, DONE_YELLOW);
        for line in &lines {
            assert!(line.starts_with(&yellow), "{line:?}");
            assert!(!line.contains('#'), "{line:?}");
            assert_eq!(strip_sgr(line).chars().count(), 100);
        }
        // Wide enough for two columns per pixel, like a mascot pixel.
        assert!(strip_sgr(&lines[0]).contains(
            "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}"
        ));
        // Too narrow for that: one column per pixel, still solid.
        let narrow = done_banner_lines(50, 10, ColorMode::TrueColor, true);
        assert_eq!(narrow.len(), 5);
        assert!(!strip_sgr(&narrow[0]).contains(&"\u{2588}".repeat(10)));
    }

    #[test]
    fn every_section_separator_is_the_border_line_with_a_set_in_title() {
        let mut state = State::new(&names(&["a"]));
        let (tx, _rx) = mpsc::channel();
        state.question = Some(("Grant access?".to_string(), "Because.".to_string(), tx));
        let frame = render_frame(&state, 120, 40, ColorMode::None, true);
        let joined = frame.join("\n");
        for title in ["PROGRESS", "QUESTION", "WHY THIS IS ASKED", "LOG"] {
            assert!(
                joined.contains(&format!("\u{2500}\u{2500}{title}")),
                "{title} is not set into a solid rule"
            );
        }
        assert!(!joined.contains("-----"), "a dashed separator is left");
        assert!(!joined.contains("\u{252c}PROGRESS"), "PROGRESS hugs the T");
    }

    #[test]
    fn the_log_pane_shows_the_worker_s_log_lines() {
        let mut state = State::new(&names(&["a"]));
        state.log.push("installed a -> /tmp/x".to_string());
        let frame = render_frame(&state, 80, 24, ColorMode::None, false);
        assert!(frame.iter().any(|l| l.contains("installed a -> /tmp/x")));
    }

    #[test]
    fn the_log_reads_oldest_to_newest_like_an_ordinary_terminal() {
        // "I'd like the log insert order to be different please, newest at
        // the bottom" -- chronological order, not the earlier newest-first
        // scheme.
        let mut state = State::new(&names(&["a"]));
        state.log.push("first".to_string());
        state.log.push("second".to_string());
        let lines = bottom_log_lines(&state, 40, '-', LOG_CONTENT_ROWS);
        let first_idx = lines.iter().position(|l| l.contains("first")).unwrap();
        let second_idx = lines.iter().position(|l| l.contains("second")).unwrap();
        assert!(
            first_idx < second_idx,
            "the older entry should come first: {lines:?}"
        );
    }

    #[test]
    fn the_log_section_is_a_fixed_height_tail() {
        // "the log staying this size through all windows" -- always exactly
        // `LOG_SECTION_ROWS`, and showing the END of the log (a tail), not
        // its start, once there is more than fits.
        let mut state = State::new(&names(&["a"]));
        for i in 0..20 {
            state.log.push(format!("line {i}"));
        }
        let lines = bottom_log_lines(&state, 40, '-', LOG_CONTENT_ROWS);
        assert_eq!(lines.len(), LOG_HEADER_ROWS + LOG_CONTENT_ROWS);
        assert!(lines.last().unwrap().contains("line 19"));
        assert!(!lines.iter().any(|l| l.contains("line 0 ")));
    }

    #[test]
    fn up_scrolls_the_log_back_and_down_returns_toward_the_live_tail() {
        let mut state = State::new(&names(&["a"]));
        for i in 0..20 {
            state.log.push(format!("line {i}"));
        }
        handle_key(&mut state, Key::Up, 80);
        handle_key(&mut state, Key::Up, 80);
        let scrolled_back = bottom_log_lines(&state, 40, '-', LOG_CONTENT_ROWS);
        assert!(scrolled_back.last().unwrap().contains("line 17"));
        handle_key(&mut state, Key::Down, 80);
        let back_toward_tail = bottom_log_lines(&state, 40, '-', LOG_CONTENT_ROWS);
        assert!(back_toward_tail.last().unwrap().contains("line 18"));
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

    /// Display columns of one rendered row: SGR escapes take none.
    fn display_cols(row: &str) -> usize {
        let mut cols = 0;
        let mut chars = row.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                cols += 1;
            }
        }
        cols
    }

    /// A row wider than the terminal wraps, pushing the frame past the last
    /// row so the terminal scrolls it up; every scroll position must fit.
    #[test]
    fn every_frame_row_fits_the_terminal_at_every_log_scroll_position() {
        let skills: Vec<&str> = vec!["ai-text-editor", "post-implementation-review", "www"];
        let mut state = State::new(&names(&skills));
        for (i, s) in [
            "chat",
            "ci-failures",
            "git-merge-resolving",
            "post-implementation-review",
        ]
        .iter()
        .cycle()
        .take(24)
        .enumerate()
        {
            state.log.push(format!(
                "installed {s} \u{2192} /Users/someone/.config/tsch-ai-skills/tmp/gui-home.{i:06}/.config/opencode/skills/{s}"
            ));
        }
        // The shape a real mcp-mode install logs: one entry carrying its own
        // indented continuation line, plus stray \r and \t for good measure.
        for _ in 0..3 {
            state.log.push(
                "installed chat -> /x/skills/chat\n              integration mode: mcp (--integration)\r\tdone"
                    .to_string(),
            );
        }
        let (tx, _rx) = std::sync::mpsc::channel();
        state.question = Some((
            "Create /Users/someone/.config/tsch-ai-worktrees as the agent worktree root?"
                .to_string(),
            "A git worktree is a second checked-out working copy of a repository.".to_string(),
            tx,
        ));
        let statuses = [
            SkillRunStatus::Pending,
            SkillRunStatus::Running,
            SkillRunStatus::Done,
            SkillRunStatus::Skipped,
        ];
        for phase in 0..6 {
            for (i, row) in state.skills.iter_mut().enumerate() {
                row.status = statuses[(i + phase) % 4];
                row.already_installed = phase % 2 == 1;
            }
            state.done = phase == 5;
            if phase == 4 {
                state.question = None;
            }
            for (cols, rows) in [(159, 42), (120, 30), (80, 24)] {
                for scroll in 0..40 {
                    state.log_scroll = scroll;
                    for unicode in [false, true] {
                        let frame = render_frame(&state, cols, rows, ColorMode::TrueColor, unicode);
                        assert_eq!(
                            frame.len(),
                            rows,
                            "{cols}x{rows} scroll {scroll}: row count"
                        );
                        for (n, row) in frame.iter().enumerate() {
                            assert_eq!(
                                display_cols(row),
                                cols,
                                "phase {phase} {cols}x{rows} scroll {scroll} unicode {unicode} row {n}: {row:?}"
                            );
                            assert!(
                                !row.chars().any(|c| c.is_control() && c != '\u{1b}'),
                                "phase {phase} {cols}x{rows} scroll {scroll} row {n} carries a control char: {row:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
