// MODE: DEV
// PACKAGE: PROD

//! Everything this release taught that a gate cannot mechanically decide --
//! printed unconditionally, every run, never folded into the pass/fail
//! count above. Each item is a judgement call a human or an agent still has
//! to make; the gates above only narrow down what to look at.

const ITEMS: &[&str] = &[
    "Version bump category: does patch/minor/major (DEVELOPMENT.md's own rule) match what actually changed -- a new skill or mod is at least minor, a removed or renamed one is major.",
    "Any stale-skill_version note above: for each, decide deliberate (a register skill's own migrate.rs testing an upgrade FROM that version) versus a fixture that needs bumping to the current version.",
    "Any newly filed bug this release cycle: release-blocking, or acceptable to ship with the register entry left open as a known issue?",
    "verify-both-shells.sh (the bash 3.2 floor): run recently against this exact tree? This check does not run it -- it is slow, and a stale pass does not catch a change made since.",
    "README.md / DEVELOPMENT.md: do they mention any skill, mod, or user-facing capability this release actually adds or removes?",
    "git status / git diff, read by eye: no secret, credential, or scratch file staged under an innocuous name.",
    "Any plugin file with no MODE marker at all (not DEV, not PROD -- just absent): confirmed harmless only because nothing currently reads it as shippable; decide whether to mark it explicitly anyway.",
    "A dedicated registers worktree, if one exists on this machine: pushed and already forwarded to origin/master? The repo-root register check above only compares what is already on disk here.",
    "The npm package's forced README/LICENSE inclusion (npm's own behaviour, not this repo's): accepted as-is, or does it warrant a documented exception somewhere a future reader would find it?",
];

pub fn print_checklist() {
    println!();
    println!("Checklist -- judgement calls this tool cannot make for you:");
    for (index, item) in ITEMS.iter().enumerate() {
        println!("  {}. {item}", index + 1);
    }
}
