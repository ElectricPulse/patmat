use super::progress::{Clone_progress, Clone_progress_widget};
use crate::target::task;

use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use vizual::widget::{Shared_widget, Widget};

#[derive(Clone)]
pub(super) struct Task {
    pub(super) path: PathBuf,
    pub(super) remote_path: String,
    pub(super) widget: Shared_widget<Widget>,
}

impl Task {
    pub fn new(path: PathBuf, remote_path: String) -> Self {
        Self {
            path,
            remote_path,
            widget: task::empty_widget(),
        }
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self, manager: &mut task::Manager<'_>) -> task::Task_result {
        let git_dir = self.path.join(".git");

        let git_directory_exists = git_dir
            .try_exists()
            .wrap_err_with(|| format!("Failed to inspect {}", git_dir.display()))?;
        if git_directory_exists && git_dir.is_dir() {
            return Ok(((), task::Status::Already_built));
        }

        let state = manager.view.render.new_state(Clone_progress::Starting);
        task::set_widget(&self.widget, Clone_progress_widget::new(state.clone())).await?;
        manager.view.refresh();

        let progress = gix::progress::tree::Root::new();
        let worker_progress = Arc::clone(&progress);
        let remote_path = self.remote_path.clone();
        let path = self.path.clone();
        let mut worker = tokio::task::spawn_blocking(move || {
            clone_repository(remote_path, path, worker_progress)
        });

        let mut refresh = tokio::time::interval(Duration::from_millis(100));
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let result = loop {
            tokio::select! {
                result = &mut worker => {
                    break result.wrap_err("The gix clone worker stopped unexpectedly")?;
                }
                _ = refresh.tick() => {
                    let snapshot = Clone_progress::from_tree(&progress);
                    if *state.load() != snapshot {
                        state.store(snapshot);
                    }
                }
            }
        };

        match result {
            Ok(()) => {
                state.store(Clone_progress::Complete);
                Ok(((), task::Status::Built))
            }
            Err(error) => {
                state.store(Clone_progress::Failed(format!("{error:#}")));
                Err(error)
            }
        }
    }
}

fn clone_repository(
    remote_path: String,
    path: PathBuf,
    progress: Arc<gix::progress::tree::Root>,
) -> Result<()> {
    let interrupt = AtomicBool::new(false);
    let mut clone = gix::prepare_clone(remote_path.as_str(), &path).wrap_err_with(|| {
        format!(
            "Failed to prepare cloning {remote_path} into {}",
            path.display()
        )
    })?;

    let fetch_progress = progress.add_child("Fetching repository");
    let (mut checkout, _) = clone
        .fetch_then_checkout(fetch_progress, &interrupt)
        .wrap_err_with(|| format!("Failed to fetch {remote_path}"))?;

    let checkout_progress = progress.add_child("Checking out repository");
    let _ = checkout
        .main_worktree(checkout_progress, &interrupt)
        .wrap_err_with(|| format!("Failed to check out {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_and_checks_out_a_local_repository() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let destination = temporary.path().join("clone");
        let progress = gix::progress::tree::Root::new();

        clone_repository(
            env!("CARGO_MANIFEST_DIR").to_string(),
            destination.clone(),
            progress,
        )?;

        assert!(destination.join(".git").is_dir());
        assert!(destination.join("Cargo.toml").is_file());
        Ok(())
    }
}
