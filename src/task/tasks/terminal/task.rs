use crate::task;
use vizual::{
    state::Store,
    widget::{Widget, Widget_trait, widgets::terminal::Terminal},
};

use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Task {
    pub(super) terminal: Terminal,
    pub(super) command: String,
    pub(super) working_dir: Option<PathBuf>,
}

impl Task {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            terminal: Terminal::new(),
            command: command.into(),
            working_dir: None,
        }
    }

    pub fn new_in_dir(command: impl Into<String>, working_dir: impl Into<PathBuf>) -> Self {
        Self {
            terminal: Terminal::new(),
            command: command.into(),
            working_dir: Some(working_dir.into()),
        }
    }

    pub fn with_terminal(terminal: Terminal, command: impl Into<String>) -> Self {
        Self {
            terminal,
            command: command.into(),
            working_dir: None,
        }
    }

    pub fn with_terminal_in_dir(
        terminal: Terminal,
        command: impl Into<String>,
        working_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            terminal,
            command: command.into(),
            working_dir: Some(working_dir.into()),
        }
    }

    pub fn widget(&self) -> Terminal {
        self.terminal.clone()
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self, widget: Store<Option<Widget>>) -> task::Task_result {
        widget.set(Some(self.terminal.clone().as_any())).await?;

        let handle = match &self.working_dir {
            Some(working_dir) => self.terminal.run_in_dir(&self.command, working_dir),
            None => self.terminal.run(&self.command),
        }?;

        handle.wait().await?;
        Ok(((), task::Status::Built))
    }
}
