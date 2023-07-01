mod args_parser;
mod command_executor;
mod screen;
mod signals;

use std::io::{self, Write};
use std::thread::sleep;

fn main() {
    // Parse command line arguments
    let args = args_parser::parse_args();

    // Register signal handler
    signals::register_signal_handler();

    // Infinite loop to run the command
    loop {
        // Check if the user has pressed CTRL+C and exit the loop if so
        if signals::is_interrupted() {
            break;
        }

        // Clear the screen
        screen::clear_screen();

        // Execute the command
        let output = command_executor::execute_command(&args.command);

        // Display the output
        print!("{}", output);

        // Flush the output to the screen immediately
        io::stdout().flush().unwrap();

        // Sleep for the specified interval
        sleep(args.interval);
    }
}
