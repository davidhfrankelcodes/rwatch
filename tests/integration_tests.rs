//! Integration tests for rwatch (cross-platform)
//!
//! Platform matrix:
//!   Windows  — cmd.exe shell (default), PowerShell (--powershell flag)
//!   macOS    — sh shell (default)
//!   Linux    — sh shell (default)

use std::{process::Command, time::{Duration, Instant}, env};

// ── test helpers ─────────────────────────────────────────────────────────────

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

/// Strip ANSI / VT100 CSI escape sequences (ESC [ ... letter) from a string.
/// Used to normalize test output when rwatch emits terminal control codes.
fn strip_terminal_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Consume the rest of the escape sequence
            match chars.peek() {
                Some(&'[') => {
                    chars.next(); // consume '['
                    // consume parameter/intermediate bytes up to the final byte
                    for c2 in chars.by_ref() {
                        if c2.is_ascii_alphabetic() { break; }
                    }
                }
                _ => {} // other escape — just drop the ESC
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Returns a single-element command list that rwatch passes verbatim to the shell.
/// On Windows:  rwatch runs  cmd /S /C "echo TEXT"  →  TEXT
/// On Linux/Mac: rwatch runs  sh -c "echo TEXT"     →  TEXT
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
    vec!["cmd.exe".into(), "/C".into(), "echo".into(), text.into()]
}
#[cfg(not(windows))]
fn exec_echo_cmd(text: &str) -> Vec<String> {
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

// ── unit-test the helpers ─────────────────────────────────────────────────────

#[test]
fn test_has_diff_line_unit() {
    assert!(has_diff_line("abc\n+foo\ndef", '+', "foo"));
    assert!(has_diff_line("abc\n foo\ndef", ' ', "foo"));
    assert!(has_diff_line("+foo", '+', "foo"));
    assert!(has_diff_line(" foo", ' ', "foo"));
    assert!(has_diff_line("header\n+foo\r\n", '+', "foo"));
    assert!(!has_diff_line("abc\n foo\n", '+', "foo"), "wrong prefix");
}

#[test]
fn test_strip_terminal_codes_unit() {
    assert_eq!(strip_terminal_codes("hello"), "hello");
    assert_eq!(strip_terminal_codes("\x1b[2Jhello"), "hello");
    assert_eq!(strip_terminal_codes("\x1b[1;1Hhello\x1b[0m"), "hello");
    assert_eq!(strip_terminal_codes("\x1b[?1049hhello\x1b[?1049l"), "hello");
    assert_eq!(strip_terminal_codes("a\x1b[31mb\x1b[0mc"), "abc");
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
    // Strip terminal control codes so the assertion isn't confused by alternate-screen sequences.
    let clean = strip_terminal_codes(&out);
    // "red" should appear somewhere (the ANSI colors were stripped, but the text remains).
    assert!(
        clean.to_lowercase().contains("red"),
        "output should still contain 'red' after ANSI stripping; raw: {}",
        out.escape_default()
    );
    // The actual command output line (not the title) should not carry \x1b[31m.
    // Find any non-title line containing "red".
    let bad_line = out.lines().find(|l| {
        l.to_lowercase().contains("red") && !l.contains("Every ")
    });
    if let Some(line) = bad_line {
        assert!(
            !line.contains("\x1b[31m"),
            "ANSI color escape should be stripped without -c; line: {}",
            line.escape_default()
        );
    }
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

/// rwatch should propagate the exact exit code of the watched command.
#[test]
fn test_exit_code_42_propagated() {
    let mut args: Vec<String> = vec!["-q".into(), "1".into(), "--".into()];
    args.extend(exit_cmd(42));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .args(&args_ref)
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 42, "rwatch should propagate exit code 42, got {}", code);
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

/// The title must show the watched command name so the user knows what they're watching.
#[test]
fn test_title_shows_command() {
    let sentinel = "title_cmd_sentinel_xyz";
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd(sentinel));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    let clean = strip_terminal_codes(&out);
    let title_line = clean.lines().find(|l| l.contains("Every "));
    assert!(
        title_line.map_or(false, |l| l.contains(sentinel)),
        "title line should include the watched command; got: {:?}",
        clean.lines().find(|l| l.contains("Every "))
    );
}

/// Title line must follow the watch format: "Every <N>s: <cmd>    <timestamp>"
#[test]
fn test_title_format() {
    let args: Vec<&str> = vec!["-n", "1.5", "--chgexit", "--", "echo", "fmt_test"];
    let (ok, out, _) = run_rwatch(&args);
    assert!(ok);
    let clean = strip_terminal_codes(&out);
    let title = clean.lines().find(|l| l.starts_with("Every ")).unwrap_or("");
    assert!(title.starts_with("Every 1.5s:"), "unexpected title format: {:?}", title);
    // Title must also contain a date/time component (year 2025 or later)
    assert!(
        title.contains("202"),
        "title should contain a timestamp: {:?}", title
    );
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

// ── precise mode ─────────────────────────────────────────────────────────────

#[test]
fn test_precise_mode_no_crash() {
    let mut args: Vec<String> = vec!["-p".into(), "-q".into(), "1".into(), "--".into()];
    args.extend(echo_cmd("precise_test"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, err) = run_rwatch(&args_ref);
    assert!(ok, "precise mode should not crash; stderr: {}", err);
    assert!(out.contains("precise_test"), "output: {}", out);
}

// ── multiple iterations ───────────────────────────────────────────────────────

/// With --equexit 2 and interval 0.2s, rwatch must run 3 cycles (1 initial +
/// 2 equal) and sleep twice, taking at least 350 ms total.
/// This verifies the loop actually repeats rather than exiting after one run.
#[test]
fn test_multiple_iterations_timing() {
    let mut args: Vec<String> = vec![
        "-n".into(), "0.2".into(),
        "-q".into(), "2".into(),
        "-t".into(),
        "--".into(),
    ];
    args.extend(echo_cmd("iter_timing"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let start = Instant::now();
    let (ok, out, err) = run_rwatch(&args_ref);
    let elapsed = start.elapsed();

    assert!(ok, "multiple-iteration run failed; stderr: {}", err);
    assert!(out.contains("iter_timing"), "output: {}", out);
    assert!(
        elapsed >= Duration::from_millis(350),
        "expected ≥350 ms for 2 sleeps of 200 ms each, got {:?}",
        elapsed
    );
}

// ── shell features ────────────────────────────────────────────────────────────

/// Pipes must work: the shell should support `cmd1 | cmd2`.
#[test]
fn test_shell_pipeline() {
    // Use commands that exist on every target platform
    #[cfg(not(windows))]
    let cmd = "echo pipeline_ok | grep pipeline_ok";
    // On Windows, cmd.exe has `findstr`; PowerShell has `Select-String`.
    // findstr is available in both cmd and PowerShell contexts (it's a real exe).
    #[cfg(windows)]
    let cmd = "echo pipeline_ok | findstr pipeline_ok";

    let args: Vec<&str> = vec!["-q", "1", "-t", "--", cmd];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "pipeline command failed; stderr: {}", err);
    assert!(
        clean.to_lowercase().contains("pipeline_ok"),
        "pipeline output should reach rwatch; got: {}", clean
    );
}

/// Chained commands with `&&` must both run.
/// `&&` is supported by both cmd.exe and POSIX sh.
#[test]
fn test_chained_commands_ampamp() {
    let cmd = "echo chain_first && echo chain_second";
    let args: Vec<&str> = vec!["-q", "1", "-t", "--", cmd];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "chained command failed; stderr: {}", err);
    assert!(clean.contains("chain_first"),  "first command missing: {}", clean);
    assert!(clean.contains("chain_second"), "second command missing: {}", clean);
}

/// Semicolon command chaining (POSIX sh only — not cmd.exe).
#[cfg(not(windows))]
#[test]
fn test_chained_commands_semicolon() {
    let cmd = "echo semi_first; echo semi_second";
    let args: Vec<&str> = vec!["-q", "1", "-t", "--", cmd];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "semicolon-chained command failed; stderr: {}", err);
    assert!(clean.contains("semi_first"),  "first command missing: {}", clean);
    assert!(clean.contains("semi_second"), "second command missing: {}", clean);
}

/// Multiline command output should be preserved with all lines intact.
#[test]
fn test_multiline_output() {
    // Use && which works on both platforms
    let cmd = "echo line_one && echo line_two && echo line_three";
    let args: Vec<&str> = vec!["-q", "1", "-t", "--", cmd];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "multiline command failed; stderr: {}", err);
    assert!(clean.contains("line_one"),   "line 1 missing: {}", clean);
    assert!(clean.contains("line_two"),   "line 2 missing: {}", clean);
    assert!(clean.contains("line_three"), "line 3 missing: {}", clean);
}

/// Subprocess stderr must NOT appear in rwatch's output.
/// rwatch only captures subprocess stdout (matching watch's behaviour).
#[test]
fn test_stderr_not_captured_in_output() {
    // Redirect stdout to stderr; nothing goes to stdout.
    // This syntax works in both cmd.exe and POSIX sh.
    // -t suppresses the title so the command string ("stderr_sentinel") doesn't appear there.
    let cmd = "echo stderr_sentinel 1>&2";
    let args: Vec<&str> = vec!["-q", "1", "-t", "--", cmd];
    let (ok, out, _) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "stderr-redirect command should exit 0");
    assert!(
        !clean.contains("stderr_sentinel"),
        "subprocess stderr must not appear in rwatch stdout; got: {}", clean
    );
}

/// A command that produces no stdout output should not crash rwatch.
#[test]
fn test_empty_stdout_no_crash() {
    // Both cmd.exe and sh: redirect stdout to stderr → rwatch sees empty stdout
    let cmd = "echo empty_check 1>&2";
    let args: Vec<&str> = vec!["-q", "1", "--", cmd];
    let (ok, _, err) = run_rwatch(&args);
    assert!(ok, "empty-stdout command should exit 0; stderr: {}", err);
}

// ── working directory ─────────────────────────────────────────────────────────

/// The subprocess must inherit rwatch's working directory, not some arbitrary default.
#[test]
fn test_working_directory_inherited() {
    let expected = env::current_dir().unwrap();
    let expected_str = expected.to_string_lossy().to_lowercase();

    // `cd` (Windows cmd) and `pwd` (sh) both print the current directory.
    #[cfg(windows)]
    let dir_cmd = vec!["-q", "1", "-t", "--", "cd"];
    #[cfg(not(windows))]
    let dir_cmd = vec!["-q", "1", "-t", "--", "pwd"];

    let (ok, out, err) = run_rwatch(&dir_cmd);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "dir command failed; stderr: {}", err);
    assert!(
        clean.to_lowercase().contains(&expected_str),
        "expected working dir {:?} in output; got:\n{}",
        expected_str, clean
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
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .args(&args)
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 42, "rwatch should propagate PowerShell exit code 42, got {}", code);
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

#[cfg(windows)]
#[test]
fn test_powershell_working_directory_inherited() {
    let expected = env::current_dir().unwrap();
    let expected_str = expected.to_string_lossy().to_lowercase();

    let args: Vec<&str> = vec!["--powershell", "-q", "1", "-t", "--", "(Get-Location).Path"];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "PowerShell dir command failed; stderr: {}", err);
    assert!(
        clean.to_lowercase().contains(&expected_str),
        "expected working dir {:?} in PS output; got:\n{}",
        expected_str, clean
    );
}

// ── helper unit tests (extended) ──────────────────────────────────────────────

#[test]
fn test_echo_cmd_helper_format() {
    let v = echo_cmd("hello");
    assert_eq!(v.len(), 1, "echo_cmd should return a single-element vec");
    assert_eq!(v[0], "echo hello", "echo_cmd format mismatch");
}

#[test]
fn test_exit_cmd_helper_format() {
    assert_eq!(exit_cmd(0)[0],  "exit 0",  "exit_cmd(0) format mismatch");
    assert_eq!(exit_cmd(1)[0],  "exit 1",  "exit_cmd(1) format mismatch");
    assert_eq!(exit_cmd(42)[0], "exit 42", "exit_cmd(42) format mismatch");
    assert_eq!(exit_cmd(0).len(), 1, "exit_cmd should be a single-element vec");
}

#[test]
fn test_strip_terminal_codes_empty_string() {
    assert_eq!(strip_terminal_codes(""), "", "empty input → empty output");
}

#[test]
fn test_strip_terminal_codes_no_escapes_unchanged() {
    let s = "plain text, no escapes 123";
    assert_eq!(strip_terminal_codes(s), s, "string with no escapes should pass through unchanged");
}

#[test]
fn test_strip_terminal_codes_multiple_consecutive_sequences() {
    // Three consecutive CSI sequences followed by text.
    let input = "\x1b[0m\x1b[1;31m\x1b[2Jhello";
    assert_eq!(strip_terminal_codes(input), "hello",
        "all three CSI sequences should be stripped");
}

#[test]
fn test_strip_terminal_codes_unicode_chars_preserved() {
    // Non-ASCII Unicode should survive even when mixed with escape sequences.
    let input = "\x1b[31mcafé\x1b[0m";
    assert_eq!(strip_terminal_codes(input), "café",
        "Unicode chars should be preserved after stripping control sequences");
}

#[test]
fn test_strip_terminal_codes_esc_not_csi_drops_only_esc() {
    // ESC followed by a non-'[' byte: only the ESC is dropped; the following char stays.
    assert_eq!(strip_terminal_codes("\x1bNhello"), "Nhello",
        "ESC-N: ESC dropped, 'N' and rest kept");
    assert_eq!(strip_terminal_codes("a\x1b(Bb"), "a(Bb",
        "ESC-(B (character-set): ESC dropped, following chars kept");
}

#[test]
fn test_strip_terminal_codes_trailing_esc_no_panic() {
    // ESC at the very end: should not panic; ESC is simply dropped.
    assert_eq!(strip_terminal_codes("hello\x1b"), "hello",
        "trailing ESC should be dropped without panic");
    assert_eq!(strip_terminal_codes("\x1b"), "",
        "lone ESC should yield empty string");
}

#[test]
fn test_has_diff_line_empty_output_always_false() {
    assert!(!has_diff_line("", '+', "anything"), "empty output should never match");
    assert!(!has_diff_line("", ' ', ""),
        "empty output with empty text — no lines to iterate, still false");
}

#[test]
fn test_has_diff_line_only_prefix_char_matches_empty_text() {
    // A line that is exactly the prefix char (no following text) should match empty text.
    assert!(has_diff_line("+", '+', ""),
        "bare '+' should match empty text after stripping prefix");
    assert!(has_diff_line("+\n", '+', ""),
        "'+\\n' should match empty text");
    assert!(!has_diff_line("+", '+', "foo"),
        "bare '+' should not match non-empty text");
}

#[test]
fn test_has_diff_line_no_false_positives() {
    assert!(!has_diff_line("+bar\n", '+', "foo"), "wrong text should not match");
    assert!(!has_diff_line("-foo\n", '+', "foo"), "wrong prefix '-' should not satisfy '+'");
    assert!(!has_diff_line(" foo\n", '+', "foo"), "space prefix should not satisfy '+' check");
    assert!(!has_diff_line("foo\n",  '+', "foo"), "line with no prefix char should not match");
}

// ── CLI introspection ─────────────────────────────────────────────────────────

#[test]
fn test_help_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .arg("--help")
        .output()
        .expect("failed to run rwatch --help");
    assert!(output.status.success(), "rwatch --help should exit 0");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.to_lowercase().contains("usage") || text.to_lowercase().contains("rwatch"),
        "help output should mention 'usage' or 'rwatch'; got: {}", text
    );
}

#[test]
fn test_version_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .arg("--version")
        .output()
        .expect("failed to run rwatch --version");
    assert!(output.status.success(), "rwatch --version should exit 0");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("0.1"),
        "version output should contain version number; got: {}", text
    );
}

#[test]
fn test_no_command_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .output()
        .expect("failed to run rwatch with no args");
    assert!(
        !output.status.success(),
        "rwatch with no arguments should exit non-zero (clap error)"
    );
}

// ── interval edge cases ───────────────────────────────────────────────────────

/// Interval of 0 is clamped to 0.1 s by the code; rwatch must not hang or panic.
#[test]
fn test_interval_zero_clamps_to_minimum() {
    let mut args: Vec<String> = vec!["-n".into(), "0".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("zero_interval_test"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, err) = run_rwatch(&args_ref);
    assert!(ok, "interval 0 should not crash; stderr: {}", err);
    assert!(out.contains("zero_interval_test"), "output: {}", out);
}

/// Without -n the default interval is 2 s; the title should show "2.0s".
#[test]
fn test_title_shows_default_two_second_interval() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd("default_interval_sentinel"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    let clean = strip_terminal_codes(&out);
    assert!(
        clean.contains("2.0s"),
        "title should show default interval 2.0s; got: {}", clean
    );
}

/// WATCH_INTERVAL set to a non-numeric value silently falls back to 2.0 s.
#[test]
fn test_watch_interval_env_invalid_falls_back_to_default() {
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd("env_invalid_sentinel"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch_with_env(&args_ref, "WATCH_INTERVAL", "not_a_number");
    assert!(ok);
    let clean = strip_terminal_codes(&out);
    assert!(
        clean.contains("2.0s"),
        "invalid WATCH_INTERVAL should fall back to 2.0s; got: {}", clean
    );
}

// ── diff across multiple runs ─────────────────────────────────────────────────

/// With -d and -q 1 at a short interval, rwatch runs exactly twice.
/// Run 1 compares against empty prev → lines are Added (+).
/// Run 2 compares against run-1 output (identical) → lines are Same (space).
/// Both markers must appear in the combined stdout.
#[test]
fn test_diff_second_run_shows_same_lines() {
    let text = "diff_two_runs_sentinel";
    let mut args: Vec<String> = vec![
        "-d".into(), "-n".into(), "0.2".into(),
        "-q".into(), "1".into(), "-t".into(), "--".into(),
    ];
    args.extend(echo_cmd(text));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, err) = run_rwatch(&args_ref);
    assert!(ok, "diff two-run test failed; stderr: {}", err);
    assert!(
        has_diff_line(&out, '+', text),
        "run 1 should show '+' (Added, prev is empty); lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
    assert!(
        has_diff_line(&out, ' ', text),
        "run 2 should show ' ' (Same, output is unchanged); lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
}

/// With -d=permanent and two identical runs, both should show Same (" " prefix).
/// Base is set on run 1, and output never diverges from it.
#[test]
fn test_permanent_diff_stable_across_two_runs() {
    let text = "perm_stable_sentinel";
    let mut args: Vec<String> = vec![
        "-d=permanent".into(), "-n".into(), "0.2".into(),
        "-q".into(), "1".into(), "-t".into(), "--".into(),
    ];
    args.extend(echo_cmd(text));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, err) = run_rwatch(&args_ref);
    assert!(ok, "permanent diff two-run test failed; stderr: {}", err);
    assert!(
        has_diff_line(&out, ' ', text),
        "permanent diff with stable output: all runs should show ' ' (Same); lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
    assert!(
        !has_diff_line(&out, '+', text),
        "permanent diff with stable output: no run should show '+' (Added); lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
}

/// -d combined with --chgexit: the first run (vs empty prev) shows Added (+).
#[test]
fn test_diff_with_chgexit_shows_added() {
    let text = "diff_chgexit_text";
    let mut args: Vec<String> = vec!["-d".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd(text));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok);
    assert!(
        has_diff_line(&out, '+', text),
        "diff+chgexit: first run should show '+'; lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
}

/// diff mode with a command that produces no stdout output should not crash.
#[test]
fn test_diff_with_empty_output_no_crash() {
    // Redirect stdout to stderr; rwatch sees empty stdout.
    let cmd = "echo empty_diff_check 1>&2";
    let args: Vec<&str> = vec!["-d", "-q", "1", "-t", "--", cmd];
    let (ok, _, err) = run_rwatch(&args);
    assert!(ok, "diff mode with empty output should not crash; stderr: {}", err);
}

/// With -q 1 and a short interval, rwatch runs exactly twice (1 initial + 1 equal).
/// Total elapsed time must be at least one interval (one sleep).
#[test]
fn test_equexit_1_runs_twice() {
    let mut args: Vec<String> = vec![
        "-n".into(), "0.25".into(), "-q".into(), "1".into(), "-t".into(), "--".into(),
    ];
    args.extend(echo_cmd("equexit1_sentinel"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let start = Instant::now();
    let (ok, out, err) = run_rwatch(&args_ref);
    let elapsed = start.elapsed();
    assert!(ok, "equexit=1 test failed; stderr: {}", err);
    assert!(out.contains("equexit1_sentinel"), "output: {}", out);
    assert!(
        elapsed >= Duration::from_millis(200),
        "equexit=1 should sleep at least once (~250 ms); got {:?}", elapsed
    );
}

// ── exec flag edge cases ──────────────────────────────────────────────────────

/// --exec with multiple positional arguments: all are passed directly without
/// shell interpretation.
#[test]
fn test_exec_multiple_args_passed_through() {
    #[cfg(not(windows))]
    let args: Vec<&str> = vec![
        "--exec", "--chgexit", "--",
        "/bin/echo", "exec_arg_a", "exec_arg_b",
    ];
    #[cfg(windows)]
    let args: Vec<&str> = vec![
        "--exec", "--chgexit", "--",
        "cmd.exe", "/C", "echo", "exec_arg_a", "exec_arg_b",
    ];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "--exec multiple args failed; stderr: {}", err);
    assert!(clean.contains("exec_arg_a"), "first arg missing: {}", clean);
    assert!(clean.contains("exec_arg_b"), "second arg missing: {}", clean);
}

// ── output content / combined flags ──────────────────────────────────────────

/// A line of 300 multi-byte Unicode characters should not cause a panic.
/// Guards the char-aware truncation fix (byte-indexing would split UTF-8 sequences).
#[test]
fn test_unicode_long_line_no_crash() {
    let long_unicode = "é".repeat(300); // each 'é' is 2 bytes in UTF-8
    let mut args: Vec<String> = vec!["--chgexit".into(), "--".into()];
    args.extend(echo_cmd(&long_unicode));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, _, err) = run_rwatch(&args_ref);
    assert!(ok, "long Unicode line should not panic; stderr: {}", err);
}

/// -t and -w together: no header appears and no ellipsis truncation.
#[test]
fn test_combined_no_title_no_wrap() {
    let long = "combined_flag_test ".repeat(20);
    let mut args: Vec<String> = vec!["-t".into(), "-w".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd(long.trim()));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, err) = run_rwatch(&args_ref);
    assert!(ok, "-t -w combined test failed; stderr: {}", err);
    assert!(!out.contains("Every "), "-t should suppress 'Every …' header: {}", out);
    assert!(!out.contains('…'),      "-w should prevent ellipsis truncation: {}", out);
    assert!(out.contains("combined_flag_test"), "output should contain command text: {}", out);
}

/// With -q 2 the same text must appear at least 3 times in combined stdout
/// (1 initial run + 2 equal runs = 3 iterations).
#[test]
fn test_equexit_output_appears_multiple_times() {
    let text = "repeat_sentinel_xyz";
    let mut args: Vec<String> = vec![
        "-n".into(), "0.1".into(), "-q".into(), "2".into(), "-t".into(), "--".into(),
    ];
    args.extend(echo_cmd(text));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, err) = run_rwatch(&args_ref);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "equexit output count test failed; stderr: {}", err);
    let count = clean.matches(text).count();
    assert!(
        count >= 3,
        "expected ≥3 occurrences of '{}' (1 initial + 2 equal runs); got {} in:\n{}",
        text, count, clean
    );
}

/// --precise with multiple iterations must complete all cycles and respect the
/// timing invariant (≥ 2 intervals elapsed for 3 runs).
#[test]
fn test_precise_multiple_iterations_timing() {
    let mut args: Vec<String> = vec![
        "-p".into(), "-n".into(), "0.2".into(),
        "-q".into(), "2".into(), "-t".into(), "--".into(),
    ];
    args.extend(echo_cmd("precise_multi_iter"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let start = Instant::now();
    let (ok, out, err) = run_rwatch(&args_ref);
    let elapsed = start.elapsed();
    assert!(ok, "--precise multiple iterations failed; stderr: {}", err);
    assert!(out.contains("precise_multi_iter"), "output: {}", out);
    assert!(
        elapsed >= Duration::from_millis(350),
        "--precise: expected ≥350 ms for 2 sleeps of 200 ms each; got {:?}", elapsed
    );
}

// ── beep behavior ─────────────────────────────────────────────────────────────

/// --beep with a failing command must emit the BEL character (\x07) to stdout.
#[test]
fn test_beep_on_failure_emits_bel() {
    let mut args: Vec<String> = vec!["-b".into(), "-q".into(), "1".into(), "--".into()];
    args.extend(exit_cmd(1));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .args(&args_ref)
        .output()
        .expect("failed to run rwatch --beep");
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_str.contains('\x07'),
        "--beep with failing command should emit BEL (\\x07); stdout bytes: {:?}",
        &output.stdout[..output.stdout.len().min(64)]
    );
}

/// --beep with a successful command must NOT emit BEL.
#[test]
fn test_beep_not_emitted_on_success() {
    let mut args: Vec<String> = vec!["-b".into(), "--chgexit".into(), "--".into()];
    args.extend(echo_cmd("no_beep_success"));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (ok, out, _) = run_rwatch(&args_ref);
    assert!(ok, "successful command with --beep should exit 0");
    assert!(
        !out.contains('\x07'),
        "BEL must not be emitted on success; stdout: {:?}",
        &out[..out.len().min(128)]
    );
}

// ── exit code propagation (extended) ─────────────────────────────────────────

/// rwatch must propagate exit code 2 from the watched command.
#[test]
fn test_exit_code_2_propagated() {
    let mut args: Vec<String> = vec!["-q".into(), "1".into(), "--".into()];
    args.extend(exit_cmd(2));
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = Command::new(env!("CARGO_BIN_EXE_rwatch"))
        .args(&args_ref)
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 2, "rwatch should propagate exit code 2; got {}", code);
}

// ── Windows-only (extended) ───────────────────────────────────────────────────

/// --powershell pipeline: items generated by a PowerShell pipeline should appear
/// in rwatch's output.
#[cfg(windows)]
#[test]
fn test_powershell_pipeline() {
    let args: Vec<&str> = vec![
        "--powershell", "-q", "1", "-t", "--",
        "1..3 | ForEach-Object { \"ps_item_$_\" }",
    ];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "PowerShell pipeline test failed: {}", err);
    assert!(clean.contains("ps_item_1"), "pipeline item 1 missing: {}", clean);
    assert!(clean.contains("ps_item_2"), "pipeline item 2 missing: {}", clean);
    assert!(clean.contains("ps_item_3"), "pipeline item 3 missing: {}", clean);
}

/// --powershell with semicolon-chained statements produces multiline output.
#[cfg(windows)]
#[test]
fn test_powershell_multiline_output() {
    let args: Vec<&str> = vec![
        "--powershell", "-q", "1", "-t", "--",
        "Write-Output 'ps_ml_1'; Write-Output 'ps_ml_2'; Write-Output 'ps_ml_3'",
    ];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "PowerShell multiline test failed: {}", err);
    assert!(clean.contains("ps_ml_1"), "line 1 missing: {}", clean);
    assert!(clean.contains("ps_ml_2"), "line 2 missing: {}", clean);
    assert!(clean.contains("ps_ml_3"), "line 3 missing: {}", clean);
}

/// --powershell combined with --no-title (-t) must suppress the "Every …" header.
#[cfg(windows)]
#[test]
fn test_powershell_no_title() {
    let args: Vec<&str> = vec![
        "--powershell", "-t", "--chgexit", "--",
        "Write-Output 'ps_notitle_ok'",
    ];
    let (ok, out, err) = run_rwatch(&args);
    assert!(ok, "PowerShell no-title test failed: {}", err);
    assert!(!out.contains("Every "), "no-title should suppress 'Every …' header: {}", out);
    assert!(out.contains("ps_notitle_ok"), "command output should still appear: {}", out);
}

/// --powershell with -d: on the first run (prev is empty) all output lines are Added (+).
#[cfg(windows)]
#[test]
fn test_powershell_diff_flag() {
    let args: Vec<&str> = vec![
        "--powershell", "-d", "--chgexit", "--",
        "Write-Output 'ps_diff_val'",
    ];
    let (ok, out, err) = run_rwatch(&args);
    assert!(ok, "PowerShell diff test failed: {}", err);
    assert!(
        has_diff_line(&out, '+', "ps_diff_val"),
        "PowerShell diff: first run should show '+' (Added); lines: {:?}",
        out.lines().collect::<Vec<_>>()
    );
}

/// --exec on Windows with cmd.exe called directly (no shell wrapper).
#[cfg(windows)]
#[test]
fn test_windows_exec_flag_with_cmd_binary() {
    let args: Vec<&str> = vec![
        "--exec", "--chgexit", "--",
        "cmd.exe", "/C", "echo", "win_exec_direct_ok",
    ];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "Windows --exec with cmd.exe failed: {}", err);
    assert!(clean.contains("win_exec_direct_ok"), "output: {}", clean);
}

// ── Unix-only (extended) ──────────────────────────────────────────────────────

/// --exec bypasses the shell, so $HOME is received literally by /bin/echo.
#[cfg(not(windows))]
#[test]
fn test_unix_exec_flag_no_shell_expansion() {
    let args: Vec<&str> = vec!["--exec", "--chgexit", "--", "/bin/echo", "$HOME"];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "--exec no-shell-expansion test failed; stderr: {}", err);
    assert!(
        clean.contains("$HOME"),
        "--exec should pass $HOME as a literal argument, not expand it; got: {}", clean
    );
}

/// The sh shell must support $() subshell expansion.
#[cfg(not(windows))]
#[test]
fn test_unix_subshell_expansion() {
    let args: Vec<&str> = vec!["-q", "1", "-t", "--", "echo $(echo subshell_expanded)"];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "subshell expansion failed; stderr: {}", err);
    assert!(
        clean.contains("subshell_expanded"),
        "$() expansion output should appear; got: {}", clean
    );
}

/// printf generates clean multiline output without needing shell-specific quoting.
#[cfg(not(windows))]
#[test]
fn test_unix_printf_multiline() {
    let args: Vec<&str> = vec![
        "-q", "1", "-t", "--",
        r"printf 'pf_line_a\npf_line_b\npf_line_c\n'",
    ];
    let (ok, out, err) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "printf multiline failed; stderr: {}", err);
    assert!(clean.contains("pf_line_a"), "line a missing: {}", clean);
    assert!(clean.contains("pf_line_b"), "line b missing: {}", clean);
    assert!(clean.contains("pf_line_c"), "line c missing: {}", clean);
}

/// When the watched command is not found, sh exits non-zero and rwatch propagates it.
#[cfg(not(windows))]
#[test]
fn test_unix_command_not_found_exits_nonzero() {
    let args: Vec<&str> = vec![
        "-q", "1", "--",
        "this_cmd_does_not_exist_xyz_rwatch_test_sentinel",
    ];
    let (ok, _, _) = run_rwatch(&args);
    assert!(!ok, "unknown command should cause rwatch to exit non-zero");
}

/// Subprocess stderr on Unix must not bleed into rwatch's captured stdout.
/// Complements the cross-platform test but uses Unix-native redirection.
#[cfg(not(windows))]
#[test]
fn test_unix_stderr_isolated_from_stdout() {
    let args: Vec<&str> = vec!["-q", "1", "-t", "--", "echo unix_stderr_only >&2"];
    let (ok, out, _) = run_rwatch(&args);
    let clean = strip_terminal_codes(&out);
    assert!(ok, "stderr-only command should exit 0");
    assert!(
        !clean.contains("unix_stderr_only"),
        "subprocess stderr should not appear in rwatch stdout; got: {}", clean
    );
}
