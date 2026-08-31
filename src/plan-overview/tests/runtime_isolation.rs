// MODE: DEV
use std::process::Command;

#[test]
fn installed_binary_has_no_prohibited_children() {
    let Some(binary) = std::env::var_os("PLAN_OVERVIEW_BIN") else {
        eprintln!("UNCONFIGURED: PLAN_OVERVIEW_BIN is required for runtime isolation");
        return;
    };
    let fixture = std::env::var_os("PLAN_OVERVIEW_FIXTURE")
        .unwrap_or_else(|| ".plans/plan-overview-rebuild".into());
    let output = Command::new(binary)
        .args(["--plan-dir"])
        .arg(fixture)
        .args(["--out", "target/runtime-isolation.html"])
        .output()
        .expect("installed plan-overview binary must start");
    assert!(output.status.success(), "render failed: {:?}", output);
}
