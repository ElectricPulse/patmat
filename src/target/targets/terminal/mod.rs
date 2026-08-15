pub mod task;

use crate::target::{Dependencies, Target};
use std::path::PathBuf;
use vizual::widget::widgets::terminal::Terminal;

pub fn new(
    name: impl Into<String>,
    command: impl Into<String>,
    dependencies: Dependencies,
) -> Target<()> {
    build(name, task::Task::new(command), dependencies)
}

pub fn new_in_dir(
    name: impl Into<String>,
    command: impl Into<String>,
    working_dir: impl Into<PathBuf>,
    dependencies: Dependencies,
) -> Target<()> {
    build(
        name,
        task::Task::new_in_dir(command, working_dir),
        dependencies,
    )
}

pub fn new_with_terminal(
    name: impl Into<String>,
    terminal: Terminal,
    command: impl Into<String>,
    dependencies: Dependencies,
) -> Target<()> {
    build(
        name,
        task::Task::with_terminal(terminal, command),
        dependencies,
    )
}

pub fn new_in_dir_with_terminal(
    name: impl Into<String>,
    terminal: Terminal,
    command: impl Into<String>,
    working_dir: impl Into<PathBuf>,
    dependencies: Dependencies,
) -> Target<()> {
    build(
        name,
        task::Task::with_terminal_in_dir(terminal, command, working_dir),
        dependencies,
    )
}

fn build(name: impl Into<String>, task: task::Task, dependencies: Dependencies) -> Target<()> {
    let working_dir = task.working_dir.clone();
    Target::new(name, working_dir, task, dependencies)
}
