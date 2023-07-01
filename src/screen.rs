use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use std::io::{stdout, Write};

pub fn clear_screen() {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All)).unwrap();
    stdout.flush().unwrap();
}
