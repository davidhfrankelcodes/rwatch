//! Basic smoke test for rwatch

use std::process::Command;

#[test]
fn rwatch_runs_and_exits() {
    // This test runs rwatch with a simple command and --chgexit so it exits after first run
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .args(["--chgexit", "--", "echo", "hello"])
        .output()
        .expect("failed to execute rwatch");
    assert!(output.status.success(), "rwatch did not exit successfully");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello"), "output did not contain expected text");
}
