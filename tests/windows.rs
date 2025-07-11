//! Integration tests for rwatch features on Windows

use std::process::Command;

fn run_rwatch(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .args(args)
        .output()
        .expect("failed to execute rwatch");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

fn echo_command(text: &str) -> Vec<String> {
    vec!["cmd".to_string(), "/C".to_string(), format!("echo {}", text)]
}

#[test]
fn rwatch_runs_and_exits() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_command("hello"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok, "rwatch did not exit successfully");
    assert!(out.to_lowercase().contains("hello"), "output did not contain expected text: {}", out);
}

#[test]
fn test_basic_echo() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_command("hello"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.to_lowercase().contains("hello"));
}

#[test]
fn test_diff_flag() {
    let mut args: Vec<String> = vec!["-d".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_command("foo"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains(" foo") || out.contains("+foo"));
}

#[test]
fn test_diff_permanent() {
    let mut args: Vec<String> = vec!["-d=permanent".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_command("bar"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains(" bar") || out.contains("+bar"));
}

#[test]
fn test_color_flag() {
    let mut args: Vec<String> = vec!["-c".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_command("\x1b[31mred\x1b[0m"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains("\x1b[31mred\x1b[0m") || out.contains("[31mred"));
}

#[test]
fn test_no_color_flag() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_command("\x1b[31mred\x1b[0m"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    let last_line = out.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    assert!(last_line.to_lowercase().contains("red"));
    assert!(!last_line.contains("\x1b[31mred\x1b[0m"), "Output line should not contain raw ANSI escape");
}

#[test]
fn test_beep_flag() {
    // Skipped: beep is not meaningful on Windows in CI
    eprintln!("Skipping beep test on Windows");
}

#[test]
fn test_equexit_flag() {
    let mut args: Vec<String> = vec!["-q".into(), "2".into(), "--".into()];
    args.extend(echo_command("same"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.to_lowercase().contains("same"));
}

#[test]
fn test_no_title_flag() {
    let mut args: Vec<String> = vec!["-t".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_command("foo"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(!out.contains("Every "));
}

#[test]
fn test_no_wrap_flag() {
    let long = "a".repeat(200);
    let mut args: Vec<String> = vec!["-w".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_command(&long));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains(&long));
}

#[test]
fn test_exec_flag() {
    let mut args: Vec<String> = vec!["-x".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_command("exec"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.to_lowercase().contains("exec"));
}

#[test]
fn test_powershell_flag() {
    // PowerShell-specific command
    let mut args: Vec<String> = vec!["--powershell".into(), "--chgexit".into(), "--".into()];
    args.push("Write-Output".into());
    args.push("pshell works!".into());
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, err) = run_rwatch(&args_ref);
    assert!(ok, "rwatch did not exit successfully: {}", err);
    assert!(out.to_lowercase().contains("pshell works!"), "output did not contain expected text: {}", out);
}
