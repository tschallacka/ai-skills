// MODE: DEV
use plan_overview::watch::{
    coalesce_events, scan_plan_dir, snapshot, watch_plan_dir, ChangeEvent, DEBOUNCE_WINDOW,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

fn temp() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "overview-watch-{}-{nonce}-{serial}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn coalescing_and_scan() {
    let root = temp();
    let outside =
        std::env::temp_dir().join(format!("overview-watch-outside-{}", std::process::id()));
    fs::write(root.join("plan.md"), "one").unwrap();
    let first = snapshot(&root).unwrap();
    fs::write(root.join("plan.md"), "two").unwrap();
    let (second, event) = scan_plan_dir(&root, &first).unwrap();
    let event = event.expect("content edit emits");
    let outside_event = ChangeEvent {
        changed: vec![outside.clone()],
        observed_at: event.observed_at + Duration::from_secs(1),
    };
    let within = ChangeEvent {
        changed: vec![root.join("other.md")],
        observed_at: event.observed_at + Duration::from_millis(10),
    };
    assert_eq!(
        coalesce_events(&[event.clone(), within], DEBOUNCE_WINDOW).len(),
        1
    );
    assert_eq!(
        coalesce_events(&[event, outside_event], DEBOUNCE_WINDOW).len(),
        2
    );
    fs::write(root.join("plan.md"), "two").unwrap();
    assert!(scan_plan_dir(&root, &second).unwrap().1.is_none());
    fs::write(&outside, "x").unwrap();
    assert!(snapshot(&root).unwrap().file_count() > 0);
    let _ = fs::remove_file(outside);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn empty_event_stream_is_quiet() {
    assert!(coalesce_events(&[], DEBOUNCE_WINDOW).is_empty());
    let _ = Instant::now();
}

#[test]
fn watcher_reports_a_real_edit() {
    let root = temp();
    fs::write(root.join("plan.md"), "one").unwrap();
    let (events, stop) = watch_plan_dir(root.clone()).unwrap();
    fs::write(root.join("plan.md"), "two").unwrap();
    let event = events
        .recv_timeout(Duration::from_secs(2))
        .expect("edit event");
    assert_eq!(event.changed, vec![root.join("plan.md")]);
    let _ = stop.send(());
    let _ = fs::remove_dir_all(root);
}
