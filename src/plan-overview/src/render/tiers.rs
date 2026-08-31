// MODE: DEV
// PACKAGE: PROD

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Full,
    Reduced,
    Minimal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TierPolicy {
    pub down_ms: f64,
    pub up_ms: f64,
    pub hysteresis_samples: usize,
}

pub const TIER_POLICY: TierPolicy = TierPolicy {
    down_ms: 24.0,
    up_ms: 16.0,
    hysteresis_samples: 3,
};

pub fn emit_tier_table() -> &'static str {
    "hysteresis: 3 samples | full: down>24ms; up<16ms; disables:none | reduced: down>32ms; up<20ms; disables:blur,edge-draw | minimal: down:none; up<20ms; disables:blur,glow,transforms,edge-draw"
}

pub struct TierController {
    tier: Tier,
    slow: usize,
    fast: usize,
    policy: TierPolicy,
}

impl TierController {
    pub fn new() -> Self {
        Self {
            tier: Tier::Full,
            slow: 0,
            fast: 0,
            policy: TIER_POLICY,
        }
    }
    pub fn with_policy(policy: TierPolicy) -> Self {
        Self {
            tier: Tier::Full,
            slow: 0,
            fast: 0,
            policy,
        }
    }
    pub fn tier(&self) -> Tier {
        self.tier
    }
    pub fn observe(&mut self, frame_ms: f64) -> Tier {
        if frame_ms > self.policy.down_ms {
            self.slow += 1;
            self.fast = 0;
        } else if frame_ms < self.policy.up_ms {
            self.fast += 1;
            self.slow = 0;
        } else {
            self.slow = 0;
            self.fast = 0;
        }
        if self.slow >= self.policy.hysteresis_samples {
            self.tier = match self.tier {
                Tier::Full => Tier::Reduced,
                Tier::Reduced => Tier::Minimal,
                Tier::Minimal => Tier::Minimal,
            };
            self.slow = 0;
        }
        if self.fast >= self.policy.hysteresis_samples {
            self.tier = match self.tier {
                Tier::Minimal => Tier::Reduced,
                Tier::Reduced => Tier::Full,
                Tier::Full => Tier::Full,
            };
            self.fast = 0;
        }
        self.tier
    }
}

impl Default for TierController {
    fn default() -> Self {
        Self::new()
    }
}
