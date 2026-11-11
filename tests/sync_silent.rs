#![cfg(feature = "sync")]

use std::process::Command;
use timeout_iterator::synchronous::TimeoutIterator;

const CHILD_ENV: &str = "TIMEOUT_ITERATOR_SILENT_CHILD";

#[test]
fn end_of_input_yields_none() {
    let mut ti = TimeoutIterator::with_iter(vec![1u32, 2].into_iter()).unwrap();
    assert_eq!(ti.next(), Some(1));
    assert_eq!(ti.next(), Some(2));
    assert_eq!(ti.next(), None);
    assert_eq!(ti.next(), None);
    assert_eq!(ti.peek(), None);
}

// Runs in a child process so we can observe what is written to the real stderr.
#[test]
fn child_end_of_input_body() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }
    let mut ti = TimeoutIterator::with_iter(vec![1u32].into_iter()).unwrap();
    assert_eq!(ti.next(), Some(1));
    assert_eq!(ti.next(), None);
    // Drop the receiver while the sender may still be running as well.
    let slow = (0..1000u32).inspect(|_| std::thread::sleep(std::time::Duration::from_millis(1)));
    let ti2 = TimeoutIterator::with_iter(slow).unwrap();
    drop(ti2);
    std::thread::sleep(std::time::Duration::from_millis(100));
}

#[test]
fn end_of_input_writes_nothing_to_stderr() {
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_end_of_input_body", "--nocapture"])
        .env(CHILD_ENV, "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "child failed: {:?}", out);
    assert!(
        out.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
