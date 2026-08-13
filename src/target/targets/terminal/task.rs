use crate::target::task;

use async_trait::async_trait;
use color_eyre::eyre::{WrapErr, bail};
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone)]
pub(crate) struct Task {
    pub(super) command: String,
    pub(super) working_dir: Option<PathBuf>,
}

impl Task {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            working_dir: None,
        }
    }
    pub fn new_in_dir(command: impl Into<String>, working_dir: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            working_dir: Some(working_dir.into()),
        }
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result {
        let mut command = Command::new("/bin/bash");
        let _ = command.arg("-c").arg(&self.command);
        if let Some(working_dir) = &self.working_dir {
            let _ = command.current_dir(working_dir);
        }

        let status = command
            .status()
            .await
            .wrap_err_with(|| format!("Failed to run {}", self.command))?;
        if !status.success() {
            bail!("Command exited with {status}");
        }

        Ok(((), task::Status::Built))
    }
}
