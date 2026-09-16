// MODE: DEV
// PACKAGE: PROD

//! ok/bad/note, matching pre-push-check.sh's own printf formatting exactly.

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
