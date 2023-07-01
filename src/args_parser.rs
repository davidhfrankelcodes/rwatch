use clap::{App, Arg};
use std::time::Duration;

pub struct Args {
    pub command: String,
    pub interval: Duration,
}

pub fn parse_args() -> Args {
    let matches = App::new("rwatch")
        .version("0.1.0")
        .author("Your Name <your.email@example.com>")
        .about("Runs a command repeatedly and shows the output")
        .arg(
            Arg::with_name("command")
                .help("The command to execute")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("interval")
                .help("The interval between command executions in seconds")
                .required(false)
                .default_value("2")
                .index(2),
        )
        .get_matches();

    let command = matches.value_of("command").unwrap().to_string();
    let interval = matches
        .value_of("interval")
        .map(|v| v.parse().expect("Failed to parse interval"))
        .unwrap_or(2);

    Args {
        command,
        interval: Duration::from_secs(interval),
    }
}
