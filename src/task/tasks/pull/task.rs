use crate::task::{self, tasks::terminal};

use async_trait::async_trait;
use color_eyre::eyre::{WrapErr, bail};
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone)]
pub(super) struct Task {
    pub(super) repo_path: PathBuf,
}

impl Task {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self) -> task::Task_result {
        // Get current commit hash silently
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()
            .await
            .wrap_err("Failed to get current commit")?;

        if !output.status.success() {
            bail!(
                "Git rev-parse failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let before_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let terminal_task = terminal::task::Task::new_in_dir("git pull", self.repo_path.clone());

        let result = {
            let result = terminal_task.run().await;
            result?
        };

        // Get new commit hash silently to determine if anything changed
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()
            .await
            .wrap_err("Failed to get new commit")?;

        let after_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Override status if nothing changed
        if before_commit == after_commit {
            return Ok(((), task::Status::Already_built));
        }

        Ok(result)
    }
}
