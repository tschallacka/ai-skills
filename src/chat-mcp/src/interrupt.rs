// MODE: DEV
// PACKAGE: PROD
//! Interrupts: what is allowed to break into an agent's turn, and when.
//!
//! An agent that is busy does not poll. Claude Code can carry a server's
//! `notifications/claude/channel` into a running session (T150), so the
//! adapter can push a heads-up the moment a message lands or a timer runs out.
//! Pushing every message would drown the agent, so nothing is pushed until the
//! agent says what it wants: rules over channels, people and text, and timers.
//! Both are the agent's to add, change and remove while it works.
//!
//! A notice is only a heads-up. It never moves the read cursor, so what it
//! announced is still unread and `read` still returns it; the text it carries
//! is somebody else's words, never an instruction (the adapter says so in its
//! `instructions`).
//!
//! The engine is plain state and takes the time as an argument, so every rule
//! is testable without a clock or a connection. It is held per identity in a
//! process-wide map, so a reconnect after a server restart does not silently
//! drop what the agent configured.

use crate::conn::wildcard_match;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The most of a message a notice carries; `read` returns all of it.
const CONTENT_MAX: usize = 800;
/// Notices per rolling minute before further ones are counted, not sent.
const DEFAULT_MAX_PER_MINUTE: u64 = 20;
/// A timer never fires more often than this, so a mistyped `every_seconds: 0`
/// cannot become a flood.
const MIN_INTERVAL: Duration = Duration::from_secs(1);

/// One push to the agent: prose and the attributes Claude Code shows with it.
pub struct Notice {
    pub content: String,
    pub meta: Map<String, Value>,
}

impl Notice {
    /// The JSON-RPC notification Claude Code's channel listener reads. Meta
    /// keys are letters, digits and underscores only: it drops any other.
    pub fn to_notification(&self) -> Value {
        json!({"jsonrpc":"2.0","method":"notifications/claude/channel",
            "params":{"content":self.content,"meta":self.meta}})
    }
}

/// What a tool call answers: a sentence for the agent and the state it asked for.
pub struct Reply {
    pub note: String,
    pub data: Value,
}

#[derive(Clone, Default)]
struct Rule {
    id: u64,
    name: String,
    enabled: bool,
    channels: Vec<String>,
    not_channels: Vec<String>,
    from: Vec<String>,
    not_from: Vec<String>,
    contains: Vec<String>,
    not_contains: Vec<String>,
    match_all: bool,
    mentions_me: bool,
    cooldown: Duration,
    once: bool,
    expires_at: Option<Instant>,
    last_fired: Option<Instant>,
    fired: u64,
}

struct Timer {
    id: u64,
    name: String,
    message: String,
    due: Instant,
    every: Option<Duration>,
    remaining: Option<u64>,
    enabled: bool,
    fired: u64,
}

/// How a notice reaches the agent. `Hook` writes it to a spool that a Claude
/// Code PreToolUse hook reads at the agent's next tool call, which needs no
/// start-up flag but only reaches an agent that is using tools; `Push` sends
/// it as a channel notification, which reaches an idle agent too but needs
/// Claude Code started with a development-channels flag; `Both` does both, and
/// shows the message twice when both work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Delivery {
    Hook,
    Push,
    Both,
}

impl Delivery {
    fn parse(text: &str) -> Result<Delivery, String> {
        match text {
            "hook" => Ok(Delivery::Hook),
            "push" => Ok(Delivery::Push),
            "both" => Ok(Delivery::Both),
            other => Err(format!(
                "delivery must be \"hook\", \"push\" or \"both\", not {other:?}"
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Delivery::Hook => "hook",
            Delivery::Push => "push",
            Delivery::Both => "both",
        }
    }
}

struct Settings {
    enabled: bool,
    max_per_minute: u64,
    snooze_until: Option<Instant>,
    delivery: Delivery,
}

pub struct Engine {
    rules: Vec<Rule>,
    timers: Vec<Timer>,
    settings: Settings,
    next_id: u64,
    recent: VecDeque<Instant>,
    suppressed: u64,
}

impl Default for Engine {
    fn default() -> Self {
        Engine {
            rules: Vec::new(),
            timers: Vec::new(),
            settings: Settings {
                enabled: true,
                max_per_minute: DEFAULT_MAX_PER_MINUTE,
                snooze_until: None,
                delivery: Delivery::Hook,
            },
            next_id: 1,
            recent: VecDeque::new(),
            suppressed: 0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Reading arguments.
// ─────────────────────────────────────────────────────────────────────────────

/// A list argument: `None` when absent (leave as it is), `Some(empty)` when
/// given as null, `[]` or "" (clear it). An array, or one string of comma- or
/// space-separated items, since a hand-written call sends either.
fn list_argument(args: &Value, key: &str) -> Result<Option<Vec<String>>, String> {
    let items: Vec<String> = match args.get(key) {
        None => return Ok(None),
        Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("{key} must be a list of strings"))
            })
            .collect::<Result<_, _>>()?,
        Some(Value::String(text)) => text
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::to_string)
            .collect(),
        Some(_) => return Err(format!("{key} must be a list of strings")),
    };
    Ok(Some(
        items
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
    ))
}

fn channel_list(args: &Value, key: &str) -> Result<Option<Vec<String>>, String> {
    let Some(items) = list_argument(args, key)? else {
        return Ok(None);
    };
    items
        .into_iter()
        .map(|item| {
            let name = if item.starts_with('#') {
                item.to_lowercase()
            } else {
                format!("#{}", item.to_lowercase())
            };
            if chat_client_rs::valid_chan(&name) {
                Ok(name)
            } else {
                Err(format!(
                    "not a channel name in {key}: {item}. A channel is '#' then lowercase letters, digits, '_' or '-'."
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn nick_list(args: &Value, key: &str) -> Result<Option<Vec<String>>, String> {
    Ok(list_argument(args, key)?.map(|items| {
        items
            .into_iter()
            .map(|item| item.trim_start_matches('@').to_string())
            .filter(|item| !item.is_empty())
            .collect()
    }))
}

fn u64_of(args: &Value, key: &str) -> Result<Option<u64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{key} must be a whole number of at least 0")),
        Some(Value::String(s)) => s
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| format!("{key} must be a whole number")),
        Some(_) => Err(format!("{key} must be a whole number")),
    }
}

fn bool_of(args: &Value, key: &str) -> Result<Option<bool>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(format!("{key} must be true or false")),
    }
}

fn text_of(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

// ─────────────────────────────────────────────────────────────────────────────
// Rules.
// ─────────────────────────────────────────────────────────────────────────────

impl Rule {
    /// Set what the call names and leave the rest, so the same code adds a rule
    /// and modifies one. A filter given as an empty list is cleared.
    fn apply(&mut self, args: &Value, now: Instant) -> Result<(), String> {
        if let Some(v) = channel_list(args, "channels")? {
            self.channels = v;
        }
        if let Some(v) = channel_list(args, "not_channels")? {
            self.not_channels = v;
        }
        if let Some(v) = nick_list(args, "from")? {
            self.from = v;
        }
        if let Some(v) = nick_list(args, "not_from")? {
            self.not_from = v;
        }
        if let Some(v) = list_argument(args, "contains")? {
            self.contains = v;
        }
        if let Some(v) = list_argument(args, "not_contains")? {
            self.not_contains = v;
        }
        if let Some(mode) = text_of(args, "match") {
            self.match_all = match mode.as_str() {
                "any" => false,
                "all" => true,
                other => return Err(format!("match must be \"any\" or \"all\", not {other:?}")),
            };
        }
        if let Some(name) = text_of(args, "name") {
            self.name = name.trim().to_string();
        }
        if let Some(v) = bool_of(args, "mentions_me")? {
            self.mentions_me = v;
        }
        if let Some(v) = bool_of(args, "once")? {
            self.once = v;
        }
        if let Some(v) = bool_of(args, "enabled")? {
            self.enabled = v;
        }
        if let Some(v) = u64_of(args, "cooldown_seconds")? {
            self.cooldown = Duration::from_secs(v);
        }
        if let Some(v) = u64_of(args, "expires_in_seconds")? {
            self.expires_at = (v > 0).then(|| now + Duration::from_secs(v));
        }
        Ok(())
    }

    fn expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|at| now >= at)
    }

    /// Every filter the rule sets must hold; one it does not set is ignored.
    fn matches(&self, me: &str, chan: &str, nick: &str, text: &str) -> bool {
        let chan = chan.to_lowercase();
        let same = |a: &String, b: &str| a.eq_ignore_ascii_case(b);
        if !self.channels.is_empty() && !self.channels.contains(&chan) {
            return false;
        }
        if self.not_channels.contains(&chan) {
            return false;
        }
        if !self.from.is_empty() && !self.from.iter().any(|f| same(f, nick)) {
            return false;
        }
        if self.not_from.iter().any(|f| same(f, nick)) {
            return false;
        }
        if self.mentions_me
            && !text
                .to_lowercase()
                .contains(&format!("@{}", me.to_lowercase()))
        {
            return false;
        }
        let hit = |p: &String| wildcard_match(p, text);
        if !self.contains.is_empty() {
            let ok = if self.match_all {
                self.contains.iter().all(hit)
            } else {
                self.contains.iter().any(hit)
            };
            if !ok {
                return false;
            }
        }
        !self.not_contains.iter().any(hit)
    }

    fn to_json(&self, now: Instant) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "enabled": self.enabled && !self.expired(now),
            "expired": self.expired(now),
            "channels": self.channels,
            "not_channels": self.not_channels,
            "from": self.from.iter().map(|n| format!("@{n}")).collect::<Vec<_>>(),
            "not_from": self.not_from.iter().map(|n| format!("@{n}")).collect::<Vec<_>>(),
            "contains": self.contains,
            "not_contains": self.not_contains,
            "match": if self.match_all { "all" } else { "any" },
            "mentions_me": self.mentions_me,
            "cooldown_seconds": self.cooldown.as_secs(),
            "once": self.once,
            "expires_in_seconds": self.expires_at.map(|at| at.saturating_duration_since(now).as_secs()),
            "fired": self.fired,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Timers.
// ─────────────────────────────────────────────────────────────────────────────

fn interval(secs: u64, key: &str) -> Result<Duration, String> {
    let d = Duration::from_secs(secs);
    if d < MIN_INTERVAL {
        return Err(format!("{key} must be at least 1"));
    }
    Ok(d)
}

impl Timer {
    fn to_json(&self, now: Instant) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "message": self.message,
            "enabled": self.enabled,
            "next_in_seconds": self.due.saturating_duration_since(now).as_secs(),
            "every_seconds": self.every.map(|d| d.as_secs()),
            "fires_left": self.remaining,
            "fired": self.fired,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The engine.
// ─────────────────────────────────────────────────────────────────────────────

impl Engine {
    pub fn delivery(&self) -> Delivery {
        self.settings.delivery
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// A message arrived on a channel this connection is in. Returns what to
    /// push, if anything: at most one notice however many rules match, and none
    /// while interrupts are off, snoozed or over their rate.
    pub fn on_message(
        &mut self,
        now: Instant,
        me: &str,
        chan: &str,
        nick: &str,
        text: &str,
    ) -> Vec<Notice> {
        if !self.settings.enabled {
            return Vec::new();
        }
        let hits: Vec<usize> = (0..self.rules.len())
            .filter(|&i| {
                let rule = &self.rules[i];
                rule.enabled
                    && !rule.expired(now)
                    && rule.matches(me, chan, nick, text)
                    && rule
                        .last_fired
                        .is_none_or(|at| now.duration_since(at) >= rule.cooldown)
            })
            .collect();
        if hits.is_empty() {
            return Vec::new();
        }
        if self.gated(now) {
            self.suppressed += 1;
            return Vec::new();
        }
        self.recent.push_back(now);
        let mut ids = Vec::new();
        let mut names = Vec::new();
        for &i in &hits {
            let rule = &mut self.rules[i];
            rule.last_fired = Some(now);
            rule.fired += 1;
            if rule.once {
                rule.enabled = false;
            }
            ids.push(rule.id.to_string());
            if !rule.name.is_empty() {
                names.push(rule.name.clone());
            }
        }
        let mut meta = Map::new();
        meta.insert("kind".into(), json!("message"));
        meta.insert("rule".into(), json!(ids.join(",")));
        if !names.is_empty() {
            meta.insert("rule_name".into(), json!(names.join(",")));
        }
        meta.insert("channel".into(), json!(chan));
        meta.insert("from".into(), json!(nick));
        meta.insert("to".into(), json!(me));
        if self.suppressed > 0 {
            meta.insert("suppressed".into(), json!(self.suppressed.to_string()));
            self.suppressed = 0;
        }
        vec![Notice {
            content: format!("{chan} <{nick}> {}", clip(text)),
            meta,
        }]
    }

    /// Snoozed, or already at the rate: the caller counts it and stays quiet.
    fn gated(&mut self, now: Instant) -> bool {
        if self.settings.snooze_until.is_some_and(|at| now < at) {
            return true;
        }
        while self
            .recent
            .front()
            .is_some_and(|&at| now.duration_since(at) >= Duration::from_secs(60))
        {
            self.recent.pop_front();
        }
        self.settings.max_per_minute > 0 && self.recent.len() as u64 >= self.settings.max_per_minute
    }

    /// Time has passed: what timers ran out. A repeating timer is rescheduled
    /// from now, not from when it was due, so a stalled connection does not
    /// come back to a burst of catch-up notices.
    pub fn tick(&mut self, now: Instant, me: &str) -> Vec<Notice> {
        if !self.settings.enabled {
            return Vec::new();
        }
        let mut notices = Vec::new();
        for timer in self.timers.iter_mut().filter(|t| t.enabled && t.due <= now) {
            timer.fired += 1;
            let mut meta = Map::new();
            meta.insert("kind".into(), json!("timer"));
            meta.insert("timer".into(), json!(timer.id.to_string()));
            if !timer.name.is_empty() {
                meta.insert("timer_name".into(), json!(timer.name));
            }
            meta.insert("to".into(), json!(me));
            notices.push(Notice {
                content: timer.message.clone(),
                meta,
            });
            match timer.every {
                Some(every) => {
                    timer.due = now + every;
                    if let Some(left) = timer.remaining.as_mut() {
                        *left = left.saturating_sub(1);
                    }
                }
                None => timer.remaining = Some(0),
            }
        }
        self.timers.retain(|t| t.remaining != Some(0));
        notices
    }

    /// One tool call against the engine.
    pub fn apply(&mut self, tool: &str, args: &Value, now: Instant) -> Result<Reply, String> {
        match tool {
            "interrupt_add" => self.rule_add(args, now),
            "interrupt_update" => self.rule_update(args, now),
            "interrupt_remove" => self.remove(args, "interrupt rule", true),
            "interrupt_list" => Ok(self.list(now, "interrupts and timers")),
            "interrupt_settings" => self.set_settings(args, now),
            "timer_set" => self.timer_set(args, now),
            "timer_update" => self.timer_update(args, now),
            "timer_cancel" => self.remove(args, "timer", false),
            other => Err(format!("not an interrupt tool: {other}")),
        }
    }

    fn id_of(args: &Value) -> Result<u64, String> {
        u64_of(args, "id")?.ok_or_else(|| "id is required".to_string())
    }

    fn rule_add(&mut self, args: &Value, now: Instant) -> Result<Reply, String> {
        let mut rule = Rule {
            enabled: true,
            ..Rule::default()
        };
        rule.apply(args, now)?;
        rule.id = self.take_id();
        let id = rule.id;
        let json = rule.to_json(now);
        self.rules.push(rule);
        Ok(Reply {
            note: format!(
                "interrupt rule {id} added. A matching message now sends you a notice (delivery: see interrupt_settings); it does not mark anything read."
            ),
            data: json,
        })
    }

    fn rule_update(&mut self, args: &Value, now: Instant) -> Result<Reply, String> {
        let id = Self::id_of(args)?;
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("no such interrupt rule: {id}"))?;
        // Apply to a copy, so a bad value refuses the whole change instead of
        // leaving the rule half-modified.
        let mut next = rule.clone();
        next.apply(args, now)?;
        *rule = next;
        Ok(Reply {
            note: format!("interrupt rule {id} updated"),
            data: rule.to_json(now),
        })
    }

    fn remove(&mut self, args: &Value, what: &str, rule: bool) -> Result<Reply, String> {
        let id = Self::id_of(args)?;
        let before = self.rules.len() + self.timers.len();
        if rule {
            self.rules.retain(|r| r.id != id);
        } else {
            self.timers.retain(|t| t.id != id);
        }
        if self.rules.len() + self.timers.len() == before {
            return Err(format!("no such {what}: {id}"));
        }
        Ok(Reply {
            note: format!("{what} {id} removed"),
            data: Value::Null,
        })
    }

    fn set_settings(&mut self, args: &Value, now: Instant) -> Result<Reply, String> {
        let enabled = bool_of(args, "enabled")?;
        let per_minute = u64_of(args, "max_per_minute")?;
        let snooze = u64_of(args, "snooze_seconds")?;
        let delivery = text_of(args, "delivery")
            .map(|text| Delivery::parse(&text))
            .transpose()?;
        if let Some(v) = enabled {
            self.settings.enabled = v;
        }
        if let Some(v) = delivery {
            self.settings.delivery = v;
        }
        if let Some(v) = per_minute {
            self.settings.max_per_minute = v;
        }
        if let Some(v) = snooze {
            self.settings.snooze_until = (v > 0).then(|| now + Duration::from_secs(v));
        }
        Ok(self.list(now, "interrupt settings"))
    }

    fn timer_set(&mut self, args: &Value, now: Instant) -> Result<Reply, String> {
        let after = u64_of(args, "after_seconds")?;
        let every = u64_of(args, "every_seconds")?;
        if after.is_none() && every.is_none() {
            return Err("timer_set needs after_seconds, every_seconds, or both".to_string());
        }
        let every = every.map(|s| interval(s, "every_seconds")).transpose()?;
        let first = match after {
            Some(s) => interval(s, "after_seconds")?,
            None => every.unwrap_or(MIN_INTERVAL),
        };
        let remaining = u64_of(args, "count")?.filter(|&count| count > 0 && every.is_some());
        let id = self.take_id();
        let name = text_of(args, "name").unwrap_or_default().trim().to_string();
        let message = text_of(args, "message")
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| {
                if name.is_empty() {
                    format!("timer {id} fired")
                } else {
                    format!("timer {name} fired")
                }
            });
        let timer = Timer {
            id,
            name,
            message,
            due: now + first,
            every,
            remaining,
            enabled: bool_of(args, "enabled")?.unwrap_or(true),
            fired: 0,
        };
        let json = timer.to_json(now);
        self.timers.push(timer);
        Ok(Reply {
            note: format!("timer {id} set"),
            data: json,
        })
    }

    fn timer_update(&mut self, args: &Value, now: Instant) -> Result<Reply, String> {
        let id = Self::id_of(args)?;
        let after = u64_of(args, "after_seconds")?;
        let every = u64_of(args, "every_seconds")?;
        let count = u64_of(args, "count")?;
        let enabled = bool_of(args, "enabled")?;
        let timer = self
            .timers
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("no such timer: {id}"))?;
        // Validate first, change after: a refused value changes nothing.
        let after = after.map(|s| interval(s, "after_seconds")).transpose()?;
        let every = match every {
            None => None,
            Some(0) => Some(None),
            Some(s) => Some(Some(interval(s, "every_seconds")?)),
        };
        if let Some(name) = text_of(args, "name") {
            timer.name = name.trim().to_string();
        }
        if let Some(message) = text_of(args, "message").filter(|m| !m.trim().is_empty()) {
            timer.message = message;
        }
        if let Some(v) = enabled {
            timer.enabled = v;
        }
        if let Some(every) = every {
            timer.every = every;
            if let Some(every) = every {
                timer.due = now + every;
            }
        }
        if let Some(after) = after {
            timer.due = now + after;
        }
        if let Some(count) = count {
            timer.remaining = (count > 0).then_some(count);
        }
        Ok(Reply {
            note: format!("timer {id} updated"),
            data: timer.to_json(now),
        })
    }

    fn list(&self, now: Instant, note: &str) -> Reply {
        let snoozed = self
            .settings
            .snooze_until
            .filter(|&at| now < at)
            .map(|at| at.duration_since(now).as_secs());
        Reply {
            note: note.to_string(),
            data: json!({
                "settings": {
                    "enabled": self.settings.enabled,
                    "max_per_minute": self.settings.max_per_minute,
                    "delivery": self.settings.delivery.name(),
                    "snoozed_for_seconds": snoozed,
                    "held_back_since_last_notice": self.suppressed,
                },
                "rules": self.rules.iter().map(|r| r.to_json(now)).collect::<Vec<_>>(),
                "timers": self.timers.iter().map(|t| t.to_json(now)).collect::<Vec<_>>(),
            }),
        }
    }
}

fn clip(text: &str) -> String {
    if text.chars().count() <= CONTENT_MAX {
        return text.to_string();
    }
    let head: String = text.chars().take(CONTENT_MAX).collect();
    format!("{head}… (cut; read returns all of it)")
}

// ─────────────────────────────────────────────────────────────────────────────
// Process-wide plumbing: one engine per identity, and where notices go.
// ─────────────────────────────────────────────────────────────────────────────

pub type SharedEngine = Arc<Mutex<Engine>>;

/// The engine for one identity, made on first use and kept for the life of the
/// process, so a connection reopened after a server restart finds the rules and
/// timers the agent had set.
pub fn engine_for(session_key: &str) -> SharedEngine {
    static ENGINES: OnceLock<Mutex<HashMap<String, SharedEngine>>> = OnceLock::new();
    let mut map = ENGINES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    map.entry(session_key.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(Engine::default())))
        .clone()
}

type Notifier = Box<dyn Fn(Value) + Send + Sync>;

static NOTIFIER: OnceLock<Notifier> = OnceLock::new();

/// Where notifications go: the transport, which owns stdout. Set once at
/// start-up; with none set (a library test) a notice is dropped.
pub fn set_notifier(send: impl Fn(Value) + Send + Sync + 'static) {
    let _ = NOTIFIER.set(Box::new(send));
}

pub fn emit(notice: &Notice) {
    if let Some(send) = NOTIFIER.get() {
        send(notice.to_notification());
    }
}

/// The most a spool file may hold; past it a notice is dropped, so an agent that
/// never uses a tool cannot grow a file without bound. The messages themselves
/// are stored by the server and `read` still returns them.
const SPOOL_MAX_BYTES: u64 = 256 * 1024;

/// Where a Claude Code hook finds what is waiting for it: one directory per
/// Claude Code session, named by the session id the harness exports, so the hook
/// (which is handed that same id) needs none of this adapter's own identity
/// logic. Where there is no such id it is the adapter's own session key.
pub fn spool_dir(state_dir: &Path, session_key: &str) -> PathBuf {
    let id = std::env::var("CLAUDE_CODE_SESSION_ID")
        .ok()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| session_key.to_string());
    state_dir.join("interrupts").join(safe_name(&id))
}

fn safe_name(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(96)
        .collect()
}

/// One spool line: when, then what the notice says, on a single line.
pub fn spool_line(notice: &Notice, at_epoch_seconds: u64) -> String {
    let second = at_epoch_seconds % 86_400;
    let clock = format!(
        "{:02}:{:02}:{:02}Z",
        second / 3600,
        second % 3600 / 60,
        second % 60
    );
    let flat = notice.content.replace(['\r', '\n'], " / ");
    match notice.meta.get("kind").and_then(Value::as_str) {
        Some("timer") => format!("[{clock}] timer: {flat}"),
        _ => format!("[{clock}] {flat}"),
    }
}

/// Append a notice to this identity's spool. Best effort: a notice that cannot be
/// written is dropped, never an error for the connection that produced it.
fn spool(state_dir: &Path, session_key: &str, notice: &Notice) {
    let dir = spool_dir(state_dir, session_key);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let file = dir.join(format!("{}.log", safe_name(session_key)));
    if std::fs::metadata(&file).is_ok_and(|m| m.len() >= SPOOL_MAX_BYTES) {
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let Ok(mut out) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
    else {
        return;
    };
    use std::io::Write;
    let _ = writeln!(out, "{}", spool_line(notice, now));
}

/// Send a notice the way the agent asked: pushed, spooled for the hook, or both.
pub fn deliver(state_dir: &Path, session_key: &str, delivery: Delivery, notice: &Notice) {
    if delivery != Delivery::Hook {
        emit(notice);
    }
    if delivery != Delivery::Push {
        spool(state_dir, session_key, notice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with(rule: Value) -> (Engine, Instant) {
        let now = Instant::now();
        let mut engine = Engine::default();
        engine
            .apply("interrupt_add", &rule, now)
            .expect("rule added");
        (engine, now)
    }

    fn fires(engine: &mut Engine, now: Instant, chan: &str, nick: &str, text: &str) -> bool {
        !engine.on_message(now, "me", chan, nick, text).is_empty()
    }

    #[test]
    fn a_rule_with_no_filter_fires_on_any_message() {
        let (mut engine, now) = engine_with(json!({}));
        assert!(fires(&mut engine, now, "#a", "x", "anything"));
    }

    #[test]
    fn no_rules_means_no_interrupts() {
        let mut engine = Engine::default();
        assert!(!fires(&mut engine, Instant::now(), "#a", "x", "hello"));
    }

    #[test]
    fn a_channel_filter_takes_names_with_or_without_the_hash_and_any_case() {
        let (mut engine, now) = engine_with(json!({"channels":["ops", "#Build"]}));
        assert!(fires(&mut engine, now, "#ops", "x", "hi"));
        assert!(fires(&mut engine, now, "#build", "x", "hi"));
        assert!(!fires(&mut engine, now, "#random", "x", "hi"));
    }

    #[test]
    fn a_people_filter_takes_at_signs_and_ignores_case() {
        let (mut engine, now) = engine_with(json!({"from":["@Alice","bob"]}));
        assert!(fires(&mut engine, now, "#a", "alice", "hi"));
        assert!(fires(&mut engine, now, "#a", "Bob", "hi"));
        assert!(!fires(&mut engine, now, "#a", "carol", "hi"));
    }

    #[test]
    fn every_filter_a_rule_sets_must_hold_together() {
        let (mut engine, now) =
            engine_with(json!({"from":["x"],"channels":["#y","#z"],"contains":["deploy"]}));
        assert!(fires(&mut engine, now, "#y", "x", "the deploy failed"));
        assert!(!fires(&mut engine, now, "#q", "x", "the deploy failed"));
        assert!(!fires(&mut engine, now, "#y", "w", "the deploy failed"));
        assert!(!fires(&mut engine, now, "#y", "x", "all quiet"));
    }

    #[test]
    fn contains_matches_any_string_by_default_and_every_one_in_all_mode() {
        let (mut any, now) = engine_with(json!({"contains":["red","blue"]}));
        assert!(fires(&mut any, now, "#a", "x", "a red car"));
        let (mut all, now) = engine_with(json!({"contains":["red","blue"],"match":"all"}));
        assert!(!fires(&mut all, now, "#a", "x", "a red car"));
        assert!(fires(&mut all, now, "#a", "x", "a red and blue car"));
    }

    #[test]
    fn strings_are_substrings_that_ignore_case_with_star_and_question_wildcards() {
        let (mut engine, now) = engine_with(json!({"contains":["BUILD *ed"]}));
        assert!(fires(&mut engine, now, "#a", "x", "the build failed today"));
        // "passed" also ends in "ed": the wildcard is what decides, so the
        // negative case has to be text with no "ed" after "build ".
        assert!(fires(&mut engine, now, "#a", "x", "the build passed"));
        assert!(!fires(&mut engine, now, "#a", "x", "the build is green"));
    }

    #[test]
    fn exclusions_veto_a_message_the_positive_filters_accept() {
        let (mut engine, now) = engine_with(json!({
            "not_from":["bot"],"not_channels":["#noise"],"not_contains":["ignore me"]}));
        assert!(fires(&mut engine, now, "#a", "x", "hi"));
        assert!(!fires(&mut engine, now, "#a", "bot", "hi"));
        assert!(!fires(&mut engine, now, "#noise", "x", "hi"));
        assert!(!fires(&mut engine, now, "#a", "x", "please IGNORE ME"));
    }

    #[test]
    fn mentions_me_wants_my_own_nick_as_an_at_mention() {
        let (mut engine, now) = engine_with(json!({"mentions_me":true}));
        assert!(!fires(&mut engine, now, "#a", "x", "hello"));
        assert!(!fires(&mut engine, now, "#a", "x", "hello me"));
        assert!(fires(&mut engine, now, "#a", "x", "ping @ME now"));
    }

    #[test]
    fn several_matching_rules_send_one_notice_naming_all_of_them() {
        let now = Instant::now();
        let mut engine = Engine::default();
        engine
            .apply("interrupt_add", &json!({"name":"one"}), now)
            .unwrap();
        engine
            .apply("interrupt_add", &json!({"name":"two","from":["x"]}), now)
            .unwrap();
        let notices = engine.on_message(now, "me", "#a", "x", "hi");
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].meta["rule"], json!("1,2"));
        assert_eq!(notices[0].meta["rule_name"], json!("one,two"));
    }

    #[test]
    fn a_notice_says_where_who_and_what_and_carries_only_letters_and_underscores_in_its_keys() {
        let (mut engine, now) = engine_with(json!({}));
        let notices = engine.on_message(now, "me", "#ops", "alice", "hello");
        let notice = &notices[0];
        assert_eq!(notice.content, "#ops <alice> hello");
        assert_eq!(notice.meta["channel"], json!("#ops"));
        assert_eq!(notice.meta["from"], json!("alice"));
        assert!(notice
            .meta
            .keys()
            .all(|k| k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')));
        let wire = notice.to_notification();
        assert_eq!(wire["method"], json!("notifications/claude/channel"));
        assert!(wire.get("id").is_none());
    }

    #[test]
    fn a_long_message_is_cut_and_says_where_the_rest_is() {
        let (mut engine, now) = engine_with(json!({}));
        let long = "x".repeat(CONTENT_MAX + 50);
        let notices = engine.on_message(now, "me", "#a", "x", &long);
        let notice = &notices[0];
        assert!(notice.content.contains("read returns all of it"));
        assert!(notice.content.len() < long.len() + 80);
    }

    #[test]
    fn a_cooldown_holds_a_rule_back_until_it_has_passed() {
        let (mut engine, now) = engine_with(json!({"cooldown_seconds":30}));
        assert!(fires(&mut engine, now, "#a", "x", "one"));
        assert!(!fires(
            &mut engine,
            now + Duration::from_secs(10),
            "#a",
            "x",
            "two"
        ));
        assert!(fires(
            &mut engine,
            now + Duration::from_secs(31),
            "#a",
            "x",
            "three"
        ));
    }

    #[test]
    fn a_once_rule_fires_once_and_turns_itself_off() {
        let (mut engine, now) = engine_with(json!({"once":true}));
        assert!(fires(&mut engine, now, "#a", "x", "one"));
        assert!(!fires(&mut engine, now, "#a", "x", "two"));
        let list = engine.apply("interrupt_list", &json!({}), now).unwrap();
        assert_eq!(list.data["rules"][0]["enabled"], json!(false));
        assert_eq!(list.data["rules"][0]["fired"], json!(1));
    }

    #[test]
    fn a_rule_with_an_expiry_stops_at_it() {
        let (mut engine, now) = engine_with(json!({"expires_in_seconds":60}));
        assert!(fires(
            &mut engine,
            now + Duration::from_secs(59),
            "#a",
            "x",
            "in time"
        ));
        assert!(!fires(
            &mut engine,
            now + Duration::from_secs(61),
            "#a",
            "x",
            "late"
        ));
    }

    #[test]
    fn a_disabled_rule_is_silent_until_it_is_enabled_again() {
        let (mut engine, now) = engine_with(json!({"enabled":false}));
        assert!(!fires(&mut engine, now, "#a", "x", "one"));
        engine
            .apply("interrupt_update", &json!({"id":1,"enabled":true}), now)
            .unwrap();
        assert!(fires(&mut engine, now, "#a", "x", "two"));
    }

    #[test]
    fn a_rule_can_be_modified_and_an_empty_list_clears_a_filter() {
        let (mut engine, now) = engine_with(json!({"from":["x"],"contains":["a"]}));
        assert!(!fires(&mut engine, now, "#a", "y", "a"));
        engine
            .apply("interrupt_update", &json!({"id":1,"from":[]}), now)
            .unwrap();
        assert!(fires(&mut engine, now, "#a", "y", "a"));
        assert!(
            !fires(&mut engine, now, "#a", "y", "b"),
            "contains stayed as it was"
        );
    }

    #[test]
    fn a_refused_update_changes_nothing() {
        let (mut engine, now) = engine_with(json!({"contains":["a"]}));
        let refused = engine.apply(
            "interrupt_update",
            &json!({"id":1,"contains":["b"],"match":"sometimes"}),
            now,
        );
        assert!(refused.is_err());
        assert!(fires(&mut engine, now, "#a", "x", "a"));
        assert!(!fires(&mut engine, now, "#a", "x", "b"));
    }

    #[test]
    fn a_removed_rule_stops_and_a_missing_one_is_refused_by_name() {
        let (mut engine, now) = engine_with(json!({}));
        engine
            .apply("interrupt_remove", &json!({"id":1}), now)
            .unwrap();
        assert!(!fires(&mut engine, now, "#a", "x", "hi"));
        let error = engine
            .apply("interrupt_remove", &json!({"id":1}), now)
            .err()
            .unwrap();
        assert!(error.contains("no such interrupt rule: 1"));
    }

    #[test]
    fn a_bad_filter_value_is_refused_by_name() {
        let now = Instant::now();
        let mut engine = Engine::default();
        for (args, needle) in [
            (json!({"channels":["Not A Channel!"]}), "not a channel name"),
            (json!({"from":[1]}), "list of strings"),
            (json!({"match":"most"}), "any"),
            (json!({"cooldown_seconds":"soon"}), "whole number"),
            (json!({"mentions_me":"yes"}), "true or false"),
        ] {
            let error = engine.apply("interrupt_add", &args, now).err().unwrap();
            assert!(error.contains(needle), "{args}: {error}");
        }
        assert!(
            engine.rules.is_empty(),
            "a refused add leaves nothing behind"
        );
    }

    #[test]
    fn a_list_may_be_one_string_of_comma_or_space_separated_items() {
        let (mut engine, now) = engine_with(json!({"channels":"#a, #b","from":"@x @y"}));
        assert!(fires(&mut engine, now, "#b", "y", "hi"));
        assert!(!fires(&mut engine, now, "#c", "y", "hi"));
    }

    #[test]
    fn the_rate_limit_holds_back_excess_and_the_next_notice_says_how_many() {
        let (mut engine, now) = engine_with(json!({}));
        engine
            .apply("interrupt_settings", &json!({"max_per_minute":2}), now)
            .unwrap();
        assert!(fires(&mut engine, now, "#a", "x", "1"));
        assert!(fires(&mut engine, now, "#a", "x", "2"));
        assert!(!fires(&mut engine, now, "#a", "x", "3"));
        assert!(!fires(&mut engine, now, "#a", "x", "4"));
        let later = now + Duration::from_secs(61);
        let fifth = engine.on_message(later, "me", "#a", "x", "5");
        assert_eq!(fifth[0].meta["suppressed"], json!("2"));
        let sixth = engine.on_message(later, "me", "#a", "x", "6");
        assert!(sixth[0].meta.get("suppressed").is_none());
    }

    #[test]
    fn zero_per_minute_means_unlimited() {
        let (mut engine, now) = engine_with(json!({}));
        engine
            .apply("interrupt_settings", &json!({"max_per_minute":0}), now)
            .unwrap();
        for _ in 0..100 {
            assert!(fires(&mut engine, now, "#a", "x", "hi"));
        }
    }

    #[test]
    fn a_snooze_mutes_messages_for_its_time_and_a_held_back_rule_is_not_spent() {
        let (mut engine, now) = engine_with(json!({"once":true}));
        engine
            .apply("interrupt_settings", &json!({"snooze_seconds":60}), now)
            .unwrap();
        assert!(!fires(&mut engine, now, "#a", "x", "snoozed"));
        assert!(fires(
            &mut engine,
            now + Duration::from_secs(61),
            "#a",
            "x",
            "awake"
        ));
    }

    #[test]
    fn switching_interrupts_off_mutes_rules_and_timers_and_on_restores_them() {
        let (mut engine, now) = engine_with(json!({}));
        engine
            .apply("timer_set", &json!({"after_seconds":5}), now)
            .unwrap();
        engine
            .apply("interrupt_settings", &json!({"enabled":false}), now)
            .unwrap();
        assert!(!fires(&mut engine, now, "#a", "x", "hi"));
        assert!(engine.tick(now + Duration::from_secs(10), "me").is_empty());
        engine
            .apply("interrupt_settings", &json!({"enabled":true}), now)
            .unwrap();
        assert!(fires(&mut engine, now, "#a", "x", "hi"));
        assert_eq!(engine.tick(now + Duration::from_secs(10), "me").len(), 1);
    }

    #[test]
    fn a_one_shot_timer_fires_once_with_its_message_and_is_gone() {
        let now = Instant::now();
        let mut engine = Engine::default();
        engine
            .apply(
                "timer_set",
                &json!({"after_seconds":30,"message":"check the build","name":"b"}),
                now,
            )
            .unwrap();
        assert!(engine.tick(now + Duration::from_secs(29), "me").is_empty());
        let fired = engine.tick(now + Duration::from_secs(30), "me");
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].content, "check the build");
        assert_eq!(fired[0].meta["kind"], json!("timer"));
        assert_eq!(fired[0].meta["timer_name"], json!("b"));
        assert!(engine.tick(now + Duration::from_secs(90), "me").is_empty());
        let list = engine.apply("interrupt_list", &json!({}), now).unwrap();
        assert_eq!(list.data["timers"], json!([]));
    }

    #[test]
    fn a_repeating_timer_repeats_from_now_and_stops_after_its_count() {
        let now = Instant::now();
        let mut engine = Engine::default();
        engine
            .apply("timer_set", &json!({"every_seconds":10,"count":2}), now)
            .unwrap();
        assert_eq!(engine.tick(now + Duration::from_secs(10), "me").len(), 1);
        // Late by a long way: one notice, not a burst of catch-up ones.
        assert_eq!(engine.tick(now + Duration::from_secs(500), "me").len(), 1);
        assert!(engine.tick(now + Duration::from_secs(600), "me").is_empty());
    }

    #[test]
    fn a_timer_needs_a_time_and_refuses_a_flood() {
        let now = Instant::now();
        let mut engine = Engine::default();
        assert!(engine.apply("timer_set", &json!({}), now).is_err());
        let error = engine
            .apply("timer_set", &json!({"every_seconds":0}), now)
            .err()
            .unwrap();
        assert!(error.contains("at least 1"));
    }

    #[test]
    fn a_timer_can_be_modified_disabled_and_cancelled() {
        let now = Instant::now();
        let mut engine = Engine::default();
        engine
            .apply("timer_set", &json!({"after_seconds":10}), now)
            .unwrap();
        engine
            .apply(
                "timer_update",
                &json!({"id":1,"after_seconds":100,"message":"later"}),
                now,
            )
            .unwrap();
        assert!(engine.tick(now + Duration::from_secs(50), "me").is_empty());
        engine
            .apply("timer_update", &json!({"id":1,"enabled":false}), now)
            .unwrap();
        assert!(engine.tick(now + Duration::from_secs(200), "me").is_empty());
        engine
            .apply("timer_update", &json!({"id":1,"enabled":true}), now)
            .unwrap();
        assert_eq!(
            engine.tick(now + Duration::from_secs(200), "me")[0].content,
            "later"
        );
        engine
            .apply("timer_set", &json!({"after_seconds":10}), now)
            .unwrap();
        engine.apply("timer_cancel", &json!({"id":2}), now).unwrap();
        assert!(engine.apply("timer_cancel", &json!({"id":2}), now).is_err());
    }

    #[test]
    fn a_repeat_can_be_dropped_from_a_timer_by_asking_for_every_zero() {
        let now = Instant::now();
        let mut engine = Engine::default();
        engine
            .apply("timer_set", &json!({"every_seconds":10}), now)
            .unwrap();
        engine
            .apply(
                "timer_update",
                &json!({"id":1,"every_seconds":0,"after_seconds":5}),
                now,
            )
            .unwrap();
        assert_eq!(engine.tick(now + Duration::from_secs(6), "me").len(), 1);
        assert!(engine.tick(now + Duration::from_secs(60), "me").is_empty());
    }

    #[test]
    fn the_list_shows_everything_the_agent_set() {
        let now = Instant::now();
        let mut engine = Engine::default();
        engine
            .apply(
                "interrupt_add",
                &json!({"name":"deploys","channels":["#ops"],"from":["@ci"],"contains":["failed"]}),
                now,
            )
            .unwrap();
        engine
            .apply(
                "timer_set",
                &json!({"every_seconds":600,"name":"stand-up"}),
                now,
            )
            .unwrap();
        let list = engine
            .apply("interrupt_list", &json!({}), now)
            .unwrap()
            .data;
        assert_eq!(list["rules"][0]["from"], json!(["@ci"]));
        assert_eq!(list["rules"][0]["channels"], json!(["#ops"]));
        assert_eq!(list["timers"][0]["every_seconds"], json!(600));
        assert_eq!(list["settings"]["enabled"], json!(true));
    }

    #[test]
    fn delivery_defaults_to_the_hook_and_can_be_changed_and_a_bad_value_is_refused() {
        let now = Instant::now();
        let mut engine = Engine::default();
        assert_eq!(engine.delivery(), Delivery::Hook);
        for (word, want) in [
            ("push", Delivery::Push),
            ("both", Delivery::Both),
            ("hook", Delivery::Hook),
        ] {
            let reply = engine
                .apply("interrupt_settings", &json!({"delivery": word}), now)
                .unwrap();
            assert_eq!(engine.delivery(), want);
            assert_eq!(reply.data["settings"]["delivery"], json!(word));
        }
        let error = engine
            .apply("interrupt_settings", &json!({"delivery":"pigeon"}), now)
            .err()
            .unwrap();
        assert!(error.contains("hook"), "{error}");
        assert_eq!(
            engine.delivery(),
            Delivery::Hook,
            "a refusal changes nothing"
        );
    }

    #[test]
    fn a_spool_line_is_the_time_then_the_notice_on_one_line() {
        let mut meta = Map::new();
        meta.insert("kind".into(), json!("message"));
        let notice = Notice {
            content: "#ops <alice> two\nlines".to_string(),
            meta,
        };
        // 1_000_000 s is 11 days and 13:46:40 after the epoch.
        assert_eq!(
            spool_line(&notice, 1_000_000),
            "[13:46:40Z] #ops <alice> two / lines"
        );
        let mut meta = Map::new();
        meta.insert("kind".into(), json!("timer"));
        let timer = Notice {
            content: "check the build".to_string(),
            meta,
        };
        assert_eq!(spool_line(&timer, 0), "[00:00:00Z] timer: check the build");
    }

    #[test]
    fn hook_delivery_spools_and_does_not_push_and_push_delivery_does_not_spool() {
        let dir = std::env::temp_dir().join(format!(
            "chat-mcp-spool-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut meta = Map::new();
        meta.insert("kind".into(), json!("message"));
        let notice = Notice {
            content: "#a <x> hello".to_string(),
            meta,
        };
        let spooled = |key: &str| -> String {
            std::fs::read_to_string(spool_dir(&dir, key).join(format!("{}.log", safe_name(key))))
                .unwrap_or_default()
        };

        deliver(&dir, "spool-test-push", Delivery::Push, &notice);
        assert_eq!(spooled("spool-test-push"), "", "push must not spool");

        deliver(&dir, "spool-test-hook", Delivery::Hook, &notice);
        deliver(&dir, "spool-test-hook", Delivery::Both, &notice);
        let text = spooled("spool-test-hook");
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(text.lines().all(|l| l.ends_with("#a <x> hello")), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_spool_name_cannot_leave_its_directory() {
        assert_eq!(safe_name("../../etc"), "______etc");
        assert_eq!(safe_name("h-4474b93c059ebd63"), "h-4474b93c059ebd63");
        assert_eq!(safe_name(&"x".repeat(500)).len(), 96);
    }

    #[test]
    fn engines_are_kept_per_identity_and_survive_being_asked_for_again() {
        let first = engine_for("interrupt-test-a");
        first
            .lock()
            .unwrap()
            .apply("interrupt_add", &json!({}), Instant::now())
            .unwrap();
        let again = engine_for("interrupt-test-a");
        assert_eq!(again.lock().unwrap().rules.len(), 1);
        assert_eq!(
            engine_for("interrupt-test-b").lock().unwrap().rules.len(),
            0
        );
    }
}
