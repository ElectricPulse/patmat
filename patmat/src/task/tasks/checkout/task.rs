use crate::task::{self, tasks::terminal};

use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, bail};
use std::path::PathBuf;
use tokio::process::Command;

use vizual::{state::Store, widget::Widget};

#[derive(Clone)]
pub(super) struct Task {
    pub(super) repo_path: PathBuf,
    pub(super) branch: String,
}

impl Task {
    pub fn new(repo_path: PathBuf, branch: String) -> Self {
        Self { repo_path, branch }
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        path.set(Some(self.repo_path.clone())).await?;
        Ok(())
    }

    async fn run(&self, widget: Store<Option<Widget>>) -> task::Task_result {
        // Check current branch silently
        let output = Command::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(&self.repo_path)
            .output()
            .await
            .wrap_err("Failed to get current branch")?;

        if !output.status.success() {
            bail!(
                "Git branch command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if current_branch == self.branch {
            return Ok(((), task::Status::Already_built));
        }

        let terminal_task = terminal::task::Task::new_in_dir(
            format!("git checkout {}", self.branch),
            self.repo_path.clone(),
        );

        let result = terminal_task.run(widget).await;
        result
    }
}
