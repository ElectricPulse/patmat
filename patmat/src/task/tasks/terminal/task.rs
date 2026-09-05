use crate::task;
use drevo::{
    state::Store,
    widget::{Widget, WidgetTrait, widgets::terminal::Terminal},
};

use async_trait::async_trait;
use color_eyre::Result;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Task {
    pub(super) terminal: Option<Terminal>,
    pub(super) command: String,
    pub(super) working_dir: Option<PathBuf>,
}

impl Task {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            terminal: None,
            command: command.into(),
            working_dir: None,
        }
    }

    pub fn new_in_dir(command: impl Into<String>, working_dir: impl Into<PathBuf>) -> Self {
        Self {
            terminal: None,
            command: command.into(),
            working_dir: Some(working_dir.into()),
        }
    }

    pub fn with_terminal(terminal: Terminal, command: impl Into<String>) -> Self {
        Self {
            terminal: Some(terminal),
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
            terminal: Some(terminal),
            command: command.into(),
            working_dir: Some(working_dir.into()),
        }
    }

    pub fn widget(&self) -> Option<Terminal> {
        self.terminal.clone()
    }
}

#[async_trait]
impl task::TaskTrait for Task {
    type Output = ();

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        if let Some(dir) = &self.working_dir {
            path.set(Some(dir.clone())).await?;
        }
        Ok(())
    }

    async fn run(&self, widget: Store<Option<Widget>>) -> task::TaskResult {
        let terminal = match &self.terminal {
            Some(terminal) => terminal.clone(),
            None => Terminal::new().await,
        };
        widget.set(Some(terminal.clone().as_any())).await?;

        let handle = match &self.working_dir {
            Some(working_dir) => terminal.run_in_dir(&self.command, working_dir).await,
            None => terminal.run(&self.command).await,
        }?;

        handle.wait().await?;
        Ok(((), task::Status::Built))
    }
}
