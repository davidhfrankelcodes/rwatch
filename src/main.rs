use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use clap::parser::ValueSource;
use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveTo, event::read};
use std::{io::{stdout, Write}, time::{Duration, Instant}, process::Command as ProcCommand, env};
use difference::Changeset;
use chrono::Local;
use regex::Regex;
use shell_words;

fn main() -> Result<()> {
    let matches = Command::new("rwatch")
        .version("0.1.0")
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
        .arg(Arg::new("command")
            .help("Command to watch")
            .required(true)
            .trailing_var_arg(true)
            .num_args(1..))
        .get_matches();

    let diff_flag = matches.contains_id("differences");
    let perm_flag = matches.get_one::<String>("differences").map(|v| v == "permanent").unwrap_or(false);
    // Determine interval_secs: if user did not provide --interval, check env var
    let interval_secs: f64 = if let Some(ValueSource::CommandLine) = matches.value_source("interval") {
        matches.get_one::<String>("interval").unwrap().parse()?

    } else if let Some(env_val) = env::var("WATCH_INTERVAL").ok() {
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

    // Collect command args as &str for join
    let cmd_vec: Vec<&str> = matches.get_many::<String>("command").unwrap()
        .map(|s| s.as_str())
        .collect();
    let cmd_str = cmd_vec.join(" ");

    let mut prev = String::new();
    let base = prev.clone();
    let mut equal_count = 0;
    let mut next = Instant::now();

    let ansi_regex = Regex::new(r"\x1b\[.*?[@-~]").unwrap();

    loop {
        next += interval;
        let output = if exec_flag {
            let ps = shell_words::split(&cmd_str)?;
            ProcCommand::new(&ps[0]).args(&ps[1..]).output()
        } else {
            #[cfg(windows)]
            let output = ProcCommand::new("cmd").arg("/C").arg(&cmd_str).output();
            #[cfg(not(windows))]
            let output = ProcCommand::new("sh").arg("-c").arg(&cmd_str).output();
            output
        };

        let output = output.map_err(|e| anyhow::anyhow!("Execution failed: {}", e))?;

        if beep && !output.status.success() {
            print!("\x07"); stdout().flush()?;
        }

        let mut stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        if !color {
            stdout_str = ansi_regex.replace_all(&stdout_str, "").to_string();
        }

        if !no_wrap {
            let (cols, _) = crossterm::terminal::size()?;
            stdout_str = stdout_str.lines()
                .map(|l| if l.len() > cols as usize { format!("{}…", &l[..cols as usize-1]) } else { l.to_string() })
                .collect::<Vec<_>>().join("\n");
        }

        execute!(stdout(), Clear(ClearType::All), MoveTo(0,0))?;
        if !no_title {
            println!("Every {:.1}s: {}    {}", interval_secs, cmd_str, Local::now().format("%Y-%m-%d %H:%M:%S"));
            println!();
        }

        if diff_flag {
            let ref_text = if perm_flag { &base } else { &prev };
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
        }

        if let Some(cycles) = equexit {
            if stdout_str == prev {
                equal_count += 1;
                if equal_count >= cycles { break; }
            } else { equal_count = 0; }
        }

        if chgexit && stdout_str != prev { break; }

        if errexit && !output.status.success() {
            eprintln!("Command error, press any key to exit..."); read()?; break;
        }

        prev = stdout_str.clone();

        if precise {
            let now = Instant::now(); if next > now { std::thread::sleep(next - now); }
        } else {
            std::thread::sleep(interval);
        }
    }

    Ok(())
}
