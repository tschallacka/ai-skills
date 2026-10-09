// MODE: DEV
// PACKAGE: PROD

//! Three report line kinds -- ok, FAIL, and note -- each with a fixed
//! prefix. Identical shape to pre-push-check's own report.rs; kept as a
//! separate small copy rather than a shared crate, the same way
//! blast-radius's own finding.rs duplicates the concept rather than
//! depending on pre-push-check for it.

pub struct Report {
    pub failures: u32,
}

impl Report {
    pub fn new() -> Self {
        Report { failures: 0 }
    }

    pub fn ok(&self, message: &str) {
        println!("  ok    {message}");
    }

    pub fn bad(&mut self, message: &str) {
        println!("  FAIL  {message}");
        self.failures += 1;
    }

    pub fn note(&self, message: &str) {
        println!("  note  {message}");
    }
}

impl Default for Report {
    fn default() -> Self {
        Self::new()
    }
}
