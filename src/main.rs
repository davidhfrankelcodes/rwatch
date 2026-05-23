use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use clap::parser::ValueSource;
use crossterm::{
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    cursor::{MoveTo, Hide, Show},
    event::read,
};
use std::{
    io::{stdout, Write},
    time::{Duration, Instant},
    process::Command as ProcCommand,
    env,
    sync::mpsc,
};
use difference::Changeset;
use chrono::Local;
use regex::Regex;
use shell_words;

fn main() -> Result<()> {
    let matches = Command::new("rwatch")
        .version("0.1.3")
        .about("execute a program periodically, showing output fullscreen")
        .arg(Arg::new("differences")
            .short('d')
            .long("differences")
            .value_name("permanent")
            .num_args(0..=1)
            .require_equals(true)
            .help("Highlight differences; use '=permanent' to keep all changes since first"))
        .arg(Arg::new("interval")
            .short('n').long("interval")
            .value_name("seconds")
            .help("Specify update interval")
            .default_value("2"))
        .arg(Arg::new("precise")
            .short('p').long("precise")
            .action(ArgAction::SetTrue)
            .help("Attempt to run command every interval"))
        .arg(Arg::new("no_title")
            .short('t').long("no-title")
            .action(ArgAction::SetTrue)
            .help("Turn off header"))
        .arg(Arg::new("beep")
            .short('b').long("beep")
            .action(ArgAction::SetTrue)
            .help("Beep if command exits non-zero"))
        .arg(Arg::new("errexit")
            .short('e').long("errexit")
            .action(ArgAction::SetTrue)
            .help("Freeze on error and exit after key press"))
        .arg(Arg::new("chgexit")
            .short('g').long("chgexit")
            .action(ArgAction::SetTrue)
            .help("Exit when output changes"))
        .arg(Arg::new("equexit")
            .short('q').long("equexit")
            .value_name("cycles")
            .help("Exit when output does not change for given cycles"))
        .arg(Arg::new("color")
            .short('c').long("color")
            .action(ArgAction::SetTrue)
            .help("Interpret ANSI color sequences"))
        .arg(Arg::new("exec")
            .short('x').long("exec")
            .action(ArgAction::SetTrue)
            .help("Pass command directly (no shell)"))
        .arg(Arg::new("no_wrap")
            .short('w').long("no-wrap")
            .action(ArgAction::SetTrue)
            .help("Turn off line wrapping"))
        .arg(Arg::new("powershell")
            .long("powershell")
            .action(ArgAction::SetTrue)
            .help("Run the command using PowerShell on Windows (instead of cmd)"))
        .arg(Arg::new("command")
            .help("Command to watch")
            .required(true)
            .trailing_var_arg(true)
            .num_args(1..))
        .get_matches();

    let diff_flag = matches.contains_id("differences");
    let perm_flag = matches.get_one::<String>("differences").map(|v| v == "permanent").unwrap_or(false);
    let interval_secs: f64 = if let Some(ValueSource::CommandLine) = matches.value_source("interval") {
        matches.get_one::<String>("interval").unwrap().parse()?
    } else if let Ok(env_val) = env::var("WATCH_INTERVAL") {
        env_val.parse().unwrap_or(2.0)
    } else {
        2.0
    };
    let interval = Duration::from_secs_f64(interval_secs.max(0.1));
    let precise = matches.get_flag("precise");
    let no_title = matches.get_flag("no_title");
    let beep = matches.get_flag("beep");
    let errexit = matches.get_flag("errexit");
    let chgexit = matches.get_flag("chgexit");
    let equexit = matches.get_one::<String>("equexit").map(|s| s.parse::<u32>().unwrap_or(0));
    let color = matches.get_flag("color");
    let exec_flag = matches.get_flag("exec");
    let no_wrap = matches.get_flag("no_wrap");
    let powershell_flag = matches.get_flag("powershell");

    let cmd_vec: Vec<&str> = matches.get_many::<String>("command").unwrap()
        .map(|s| s.as_str())
        .collect();
    let cmd_str = cmd_vec.join(" ");

    let cwd = env::current_dir()?;

    // Enter alternate screen (like real watch). Non-fatal when stdout is not a TTY (e.g., tests).
    let entered_alt = execute!(stdout(), EnterAlternateScreen, Hide).is_ok();

    // Ctrl+C sends on this channel so the loop can clean up before exiting.
    let (ctrlc_tx, ctrlc_rx) = mpsc::channel::<()>();
    let _ = ctrlc::set_handler(move || { let _ = ctrlc_tx.send(()); });

    let mut prev = String::new();
    let mut base: Option<String> = None;
    let mut equal_count = 0u32;
    let mut next = Instant::now();
    let ansi_regex = Regex::new(r"\x1b\[.*?[@-~]").unwrap();
    // loop always runs at least once; 0 is overwritten before being read
    #[allow(unused_assignments)]
    let mut last_status = 0i32;

    // IIFE so that `?` errors still reach the cleanup below.
    let loop_result: Result<()> = (|| {
        loop {
            next += interval;

            // Check for Ctrl+C that arrived before or during the previous sleep.
            match ctrlc_rx.try_recv() {
                Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {}
            }

            let output = if exec_flag {
                let ps = shell_words::split(&cmd_str)?;
                ProcCommand::new(&ps[0]).args(&ps[1..]).current_dir(&cwd).output()
            } else {
                #[cfg(windows)]
                let output = if powershell_flag {
                    ProcCommand::new("powershell.exe")
                        .args(&["-Command", &cmd_str])
                        .current_dir(&cwd)
                        .output()
                } else {
                    // /S makes cmd.exe correctly strip both outer quotes from the argument.
                    ProcCommand::new("cmd")
                        .args(&["/S", "/C", &cmd_str])
                        .current_dir(&cwd)
                        .output()
                };
                #[cfg(not(windows))]
                let output = ProcCommand::new("sh")
                    .arg("-c")
                    .arg(&cmd_str)
                    .current_dir(&cwd)
                    .output();
                output
            };

            let output = output.map_err(|e| anyhow::anyhow!("Execution failed: {}", e))?;
            let status_code = output.status.code().unwrap_or(1);
            last_status = status_code;

            if beep && !output.status.success() {
                print!("\x07"); stdout().flush()?;
            }

            let mut stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
            if !color {
                stdout_str = ansi_regex.replace_all(&stdout_str, "").to_string();
            }

            if !no_wrap {
                if let Ok((cols, _)) = crossterm::terminal::size() {
                    let limit = (cols as usize).saturating_sub(1);
                    stdout_str = stdout_str.lines()
                        .map(|l| {
                            let chars: Vec<char> = l.chars().collect();
                            if chars.len() > cols as usize {
                                format!("{}…", chars[..limit].iter().collect::<String>())
                            } else {
                                l.to_string()
                            }
                        })
                        .collect::<Vec<_>>().join("\n");
                }
            }

            // Clear within the alternate screen buffer (or the main buffer if alt not supported).
            let _ = execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0));
            if !no_title {
                println!("Every {:.1}s: {}    {}", interval_secs, cmd_str, Local::now().format("%Y-%m-%d %H:%M:%S"));
                println!();
            }

            if perm_flag && base.is_none() {
                base = Some(stdout_str.clone());
            }

            if diff_flag {
                let base_str = base.as_deref().unwrap_or("");
                let ref_text = if perm_flag { base_str } else { prev.as_str() };
                let changes = Changeset::new(ref_text, &stdout_str, "\n");
                for diff in changes.diffs {
                    match diff {
                        difference::Difference::Same(ref x) => for line in x.lines() { println!(" {}", line); },
                        difference::Difference::Add(ref x) => for line in x.lines() { println!("+{}", line); },
                        difference::Difference::Rem(ref x) => for line in x.lines() { println!("-{}", line); },
                    }
                }
            } else {
                print!("{}", stdout_str);
                stdout().flush()?;
            }

            if let Some(cycles) = equexit {
                if stdout_str == prev {
                    equal_count += 1;
                    if equal_count >= cycles { break; }
                } else { equal_count = 0; }
            }
            if chgexit && stdout_str != prev { break; }
            if errexit && !output.status.success() {
                println!("\nCommand error, press any key to exit...");
                stdout().flush()?;
                read()?;
                break;
            }
            prev = stdout_str.clone();

            // Sleep for the interval; wake early if Ctrl+C arrives.
            let sleep_dur = if precise {
                next.saturating_duration_since(Instant::now())
            } else {
                interval
            };
            match ctrlc_rx.recv_timeout(sleep_dur) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        Ok(())
    })();

    // Always restore the terminal, even if the loop returned an error.
    if entered_alt {
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
    }

    loop_result?;

    if last_status != 0 {
        std::process::exit(last_status);
    }
    Ok(())
}
