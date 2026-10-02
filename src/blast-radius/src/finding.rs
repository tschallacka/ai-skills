// MODE: DEV
// PACKAGE: PROD
//! One rendered output line plus its counting/stream disposition: fail/warn
//! go to stderr and are counted, while plain ok/note lines go to stdout and
//! are not.

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
