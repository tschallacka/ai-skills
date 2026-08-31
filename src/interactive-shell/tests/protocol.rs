use std::path::Path;

#[test]
fn protocol_test_target_exists() {
    assert!(Path::new("src/lib.rs").exists());
}
