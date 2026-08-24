pub mod task;

use std::path::PathBuf;
use vizual::widget::widgets::terminal::Terminal;

use crate::task::Task;

pub fn new(command: impl Into<String>) -> Task<()> {
    build(task::Task::new(command))
}

pub fn new_in_dir(command: impl Into<String>, working_dir: impl Into<PathBuf>) -> Task<()> {
    build(task::Task::new_in_dir(command, working_dir))
}

pub fn with_terminal(terminal: Terminal, command: impl Into<String>) -> Task<()> {
    build(task::Task::with_terminal(terminal, command))
}

pub fn with_terminal_in_dir(
    terminal: Terminal,
    command: impl Into<String>,
    working_dir: impl Into<PathBuf>,
) -> Task<()> {
    build(task::Task::with_terminal_in_dir(terminal, command, working_dir))
}

fn build(task: task::Task) -> Task<()> {
    Task::new(task)
}
