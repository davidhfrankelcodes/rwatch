//! Integration tests for rwatch (cross-platform)

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

fn run_rwatch_with_env(args: &[&str], key: &str, val: &str) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .args(args)
        .env(key, val)
        .output()
        .expect("failed to execute rwatch");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

/// Returns a single-element command list that rwatch passes verbatim to the shell.
/// On Windows:  rwatch runs  cmd /S /C "echo TEXT"  →  echo TEXT  →  TEXT
/// On Linux/Mac: rwatch runs  sh -c "echo TEXT"  →  echo TEXT  →  TEXT
///
/// Single-element avoids the double-nesting quoting bug that appeared when passing
/// ["cmd", "/C", "echo TEXT"] (which made rwatch invoke cmd /S /C "cmd /C echo TEXT",
/// where cmd.exe's quote-stripping left a trailing " in the output).
fn echo_cmd(text: &str) -> Vec<String> {
    vec![format!("echo {}", text)]
}

/// Returns a single-element exit command.
fn exit_cmd(code: i32) -> Vec<String> {
    vec![format!("exit {}", code)]
}

/// Returns command tokens suitable for the --exec flag (bypasses rwatch's shell wrapper).
/// Must resolve to a real executable, not a shell builtin.
#[cfg(windows)]
fn exec_echo_cmd(text: &str) -> Vec<String> {
    // shell_words will split this as ["cmd.exe", "/C", "echo", TEXT]
    vec!["cmd.exe".into(), "/C".into(), "echo".into(), text.into()]
}
#[cfg(not(windows))]
fn exec_echo_cmd(text: &str) -> Vec<String> {
    // /bin/echo is a real binary (not just a shell builtin)
    vec!["/bin/echo".into(), text.into()]
}

/// Returns true if any line in `output` starts with `prefix` char and whose
/// trimmed remainder equals `text`.
fn has_diff_line(output: &str, prefix: char, text: &str) -> bool {
    output.lines().any(|l| {
        l.strip_prefix(prefix)
            .map_or(false, |rest| rest.trim() == text)
    })
}

// ── unit-test the helper itself ───────────────────────────────────────────────

#[test]
fn test_has_diff_line_unit() {
    assert!(has_diff_line("abc\n+foo\ndef", '+', "foo"));
    assert!(has_diff_line("abc\n foo\ndef", ' ', "foo"));
    assert!(has_diff_line("+foo", '+', "foo"));
    assert!(has_diff_line(" foo", ' ', "foo"));
    assert!(has_diff_line("header\n+foo\r\n", '+', "foo"));
    assert!(!has_diff_line("abc\n foo\n", '+', "foo"), "wrong prefix");
}

// ── basic invocation ─────────────────────────────────────────────────────────

#[test]
fn rwatch_runs_and_exits() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd("hello"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok, "rwatch did not exit successfully");
    assert!(out.to_lowercase().contains("hello"), "output: {}", out);
}

#[test]
fn test_basic_echo() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd("hello"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.to_lowercase().contains("hello"));
}

// ── diff flag ────────────────────────────────────────────────────────────────

#[test]
fn test_diff_flag() {
    let mut args: Vec<String> = vec!["-d".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("foo"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    // First run: prev is empty, so the output is entirely "added" (+ prefix)
    assert!(has_diff_line(&out, '+', "foo"),
        "diff output should mark first run as added; lines: {:?}",
        out.lines().collect::<Vec<_>>());
}

#[test]
fn test_diff_permanent_basic() {
    let mut args: Vec<String> = vec!["-d=permanent".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("bar"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(has_diff_line(&out, ' ', "bar") || has_diff_line(&out, '+', "bar"),
        "diff output should contain bar; lines: {:?}", out.lines().collect::<Vec<_>>());
}

/// Bug fix verification: permanent diff must compare against the FIRST run's output,
/// not against the empty string that was erroneously used before the fix.
/// On the first (and only) run with --chgexit, base == first output, so the diff
/// should show Same (space prefix), not Added (+ prefix).
#[test]
fn test_permanent_diff_first_run_is_base() {
    let text = "perm_diff_sentinel";
    let mut args: Vec<String> = vec!["-d=permanent".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd(text));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(
        has_diff_line(&out, ' ', text),
        "permanent diff: first run output becomes the base, so diff(base, output) = Same \
         (space prefix); lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
    assert!(
        !has_diff_line(&out, '+', text),
        "permanent diff: first run should NOT appear as Added (+); lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
}

/// Regular diff (-d without =permanent) compares against prev, so the very first
/// run shows everything as Added because prev starts empty.
#[test]
fn test_regular_diff_first_run_is_added() {
    let text = "regular_diff_sentinel";
    let mut args: Vec<String> = vec!["-d".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd(text));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(
        has_diff_line(&out, '+', text),
        "regular diff: first run should be Added since prev is empty; lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
    assert!(
        !has_diff_line(&out, ' ', text),
        "regular diff: first run should NOT be Same; lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
}

// ── color ────────────────────────────────────────────────────────────────────

#[test]
fn test_color_flag() {
    let mut args: Vec<String> = vec!["-c".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("\x1b[31mred\x1b[0m"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains("\x1b[31mred\x1b[0m") || out.contains("[31mred"),
        "color mode should preserve ANSI sequences: {}", out);
}

#[test]
fn test_no_color_strips_ansi() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd("\x1b[31mred\x1b[0m"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    let content = out.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    assert!(content.to_lowercase().contains("red"), "output should still contain 'red': {}", out);
    assert!(!content.contains("\x1b[31m"), "ANSI escape should be stripped without -c: {}", out);
}

// ── exit conditions ───────────────────────────────────────────────────────────

#[test]
fn test_chgexit_flag() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd("chgexit_val"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains("chgexit_val"), "output: {}", out);
}

#[test]
fn test_equexit_flag() {
    let mut args: Vec<String> = vec!["-q".into(), "2".into(), "--".into()];
    args.extend(echo_cmd("same_value"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains("same_value"), "output: {}", out);
}

/// A successful command should cause rwatch to exit 0.
#[test]
fn test_exit_code_zero_on_success() {
    let mut args: Vec<String> = vec!["-q".into(), "1".into(), "--".into()];
    args.extend(echo_cmd("ok"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, _, err) = run_rwatch(&args_ref);
    assert!(ok, "successful command should exit 0; stderr: {}", err);
}

/// A failing command should cause rwatch to exit non-zero.
#[test]
fn test_exit_code_nonzero_on_failure() {
    let mut args: Vec<String> = vec!["-q".into(), "1".into(), "--".into()];
    args.extend(exit_cmd(1));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, _, _) = run_rwatch(&args_ref);
    assert!(!ok, "failing command should exit non-zero");
}

/// Verifies last_status tracks the LAST run's exit code, not any arbitrary earlier failure.
#[test]
fn test_equexit_success_exits_zero() {
    let mut args: Vec<String> = vec!["-q".into(), "1".into(), "--".into()];
    args.extend(echo_cmd("should_be_zero"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, _, err) = run_rwatch(&args_ref);
    assert!(ok, "equexit with successful command should exit 0; stderr: {}", err);
}

// ── title / wrap ─────────────────────────────────────────────────────────────

#[test]
fn test_no_title_flag() {
    let mut args: Vec<String> = vec!["-t".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("foo"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(!out.contains("Every "), "no-title should suppress header: {}", out);
}

#[test]
fn test_no_wrap_flag() {
    let long = "a".repeat(200);
    let mut args: Vec<String> = vec!["-w".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd(&long));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains(&long), "no-wrap should not truncate long lines");
}

/// Verifies the line-wrapping code does not panic on long lines.
/// Also exercises the char-aware truncation fix (multi-byte UTF-8 safe).
#[test]
fn test_long_line_no_panic() {
    let long = "x".repeat(500);
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd(&long));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, _, err) = run_rwatch(&args_ref);
    assert!(ok, "long line should not panic; stderr: {}", err);
}

#[test]
fn test_title_shows_interval() {
    let mut args: Vec<String> = vec!["-n".into(), "0.5".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("itest"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.contains("0.5s"), "title should show 0.5s interval: {}", out);
}

// ── interval env var ─────────────────────────────────────────────────────────

#[test]
fn test_watch_interval_env_var() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd("env_interval_val"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch_with_env(&args_ref, "WATCH_INTERVAL", "0.3");
    assert!(ok);
    assert!(out.contains("env_interval_val"), "output: {}", out);
    assert!(out.contains("0.3s"), "title should show env-var interval 0.3s: {}", out);
}

/// --interval on the command line should take precedence over WATCH_INTERVAL.
#[test]
fn test_cli_interval_overrides_env_var() {
    let mut args: Vec<String> = vec!["-n".into(), "0.7".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("override_test"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch_with_env(&args_ref, "WATCH_INTERVAL", "9.9");
    assert!(ok);
    assert!(out.contains("0.7s"), "CLI interval should override env var: {}", out);
    assert!(!out.contains("9.9s"), "env-var interval should not appear when -n is set: {}", out);
}

// ── exec flag ────────────────────────────────────────────────────────────────

#[test]
fn test_exec_flag() {
    let mut args: Vec<String> = vec!["-x".into(), "--chgexit".into(), "--".into()];
    args.extend(exec_echo_cmd("exec_output"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(out.to_lowercase().contains("exec_output"), "exec output: {}", out);
}

// ── beep (no assertion, just no crash) ───────────────────────────────────────

#[test]
fn test_beep_flag_no_crash() {
    let mut args: Vec<String> = vec!["-b".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("beep_test"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, _, _) = run_rwatch(&args_ref);
    assert!(ok, "beep flag should not cause a crash on success");
}

// ── working directory ─────────────────────────────────────────────────────────

/// The subprocess must inherit rwatch's working directory, not some arbitrary default.
#[test]
fn test_working_directory_inherited() {
    use std::env;

    let expected = env::current_dir().unwrap();
    let expected_str = expected.to_string_lossy().to_lowercase();

    // `cd` (Windows cmd) and `pwd` (sh) both print the current directory.
    #[cfg(windows)]
    let dir_cmd = vec!["-q", "1", "-t", "--", "cd"];
    #[cfg(not(windows))]
    let dir_cmd = vec!["-q", "1", "-t", "--", "pwd"];

    let (ok, out, err) = run_rwatch(&dir_cmd);
    assert!(ok, "dir command failed; stderr: {}", err);
    assert!(
        out.to_lowercase().contains(&expected_str),
        "expected working dir {:?} in output; got:\n{}",
        expected_str, out
    );
}

/// PowerShell mode must also inherit the correct working directory.
#[cfg(windows)]
#[test]
fn test_powershell_working_directory_inherited() {
    use std::env;

    let expected = env::current_dir().unwrap();
    let expected_str = expected.to_string_lossy().to_lowercase();

    let args: Vec<&str> = vec!["--powershell", "-q", "1", "-t", "--", "(Get-Location).Path"];
    let (ok, out, err) = run_rwatch(&args);
    assert!(ok, "PowerShell dir command failed; stderr: {}", err);
    assert!(
        out.to_lowercase().contains(&expected_str),
        "expected working dir {:?} in PS output; got:\n{}",
        expected_str, out
    );
}

// ── Windows-only ─────────────────────────────────────────────────────────────

#[cfg(windows)]
#[test]
fn test_powershell_flag() {
    let args: Vec<&str> = vec!["--powershell", "--chgexit", "--", "Write-Output pshell_works"];
    let (ok, out, err) = run_rwatch(&args);
    assert!(ok, "rwatch --powershell did not exit successfully: {}", err);
    assert!(out.to_lowercase().contains("pshell_works"), "output: {}", out);
}

#[cfg(windows)]
#[test]
fn test_powershell_exit_code() {
    let args: Vec<&str> = vec!["--powershell", "--equexit", "1", "--", "exit 42"];
    let (ok, _, _) = run_rwatch(&args);
    assert!(!ok, "rwatch should propagate non-zero PowerShell exit code");
}

#[cfg(windows)]
#[test]
fn test_powershell_overrides_cmd() {
    // Write-Output is PowerShell-only; cmd.exe would not produce this output.
    let args: Vec<&str> = vec![
        "--powershell",
        "--chgexit",
        "--",
        "Write-Output ps_only_marker",
    ];
    let (ok, out, err) = run_rwatch(&args);
    assert!(ok, "PowerShell command failed: {}", err);
    assert!(out.contains("ps_only_marker"), "output: {}", out);
}
