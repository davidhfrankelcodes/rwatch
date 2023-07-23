// screen.rs

use crossterm::execute;
use crossterm::cursor::MoveTo;
use crossterm::terminal::{Clear, ClearType};
use std::io::{stdout, Write};

pub fn clear_screen() {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All)).unwrap();
    stdout.flush().unwrap();
}

pub fn set_cursor_to_top() {
    let mut stdout = stdout();
    execute!(stdout, MoveTo(0, 0)).unwrap();
    stdout.flush().unwrap();
}
