// MODE: DEV
// PACKAGE: PROD
//! One rendered output line plus its counting/stream disposition, mirroring
//! bash's own `report_fail`/`report_warn` (stderr, counted) versus the
//! plain `ok:`/`note:` printfs (stdout, uncounted).

pub enum Level {
    Fail,
    Warn,
}

pub struct Line {
    pub text: String,
    pub to_stderr: bool,
    pub level: Option<Level>,
}

pub fn fail(message: impl Into<String>) -> Line {
    Line {
        text: format!("FAIL: {}", message.into()),
        to_stderr: true,
        level: Some(Level::Fail),
    }
}

pub fn warn(message: impl Into<String>) -> Line {
    Line {
        text: format!("WARN: {}", message.into()),
        to_stderr: true,
        level: Some(Level::Warn),
    }
}

pub fn ok(consequence: &str) -> Line {
    Line {
        text: format!("ok:   {consequence}"),
        to_stderr: false,
        level: None,
    }
}

pub fn note(message: impl Into<String>) -> Line {
    Line {
        text: format!("note: {}", message.into()),
        to_stderr: false,
        level: None,
    }
}
