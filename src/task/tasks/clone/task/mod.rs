use super::progress::{Clone_progress, Clone_progress_widget};
use crate::task;

use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use vizual::{
    state::{State, Store},
    widget::{Widget, Widget_trait, widgets::positioning::anchor::Anchor},
};

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(super) struct Task {
    pub(super) path: PathBuf,
    pub(super) remote_path: String,
    progress: Store<Clone_progress>,
}

impl Task {
    pub fn new(path: PathBuf, remote_path: String) -> Self {
        Self {
            path,
            remote_path,
            progress: Store::new(Clone_progress::Starting),
        }
    }

    pub(super) fn widget(&self) -> Clone_progress_widget {
        Clone_progress_widget::new(self.progress.clone())
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self, widget: Store<Option<Widget>>) -> task::Task_result {
        *widget.write().await? = Some(Anchor::middle(self.widget()).as_any());

        let git_dir = self.path.join(".git");

        let git_directory_exists = git_dir
            .try_exists()
            .wrap_err_with(|| format!("Failed to inspect {}", git_dir.display()))?;
        if git_directory_exists && git_dir.is_dir() {
            *self.progress.write().await? = Clone_progress::Complete;
            return Ok(((), task::Status::Already_built));
        }

        *self.progress.write().await? = Clone_progress::Starting;

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
                    if *self.progress.read().await? != snapshot {
                        *self.progress.write().await? = snapshot;
                    }
                }
            }
        };

        match result {
            Ok(()) => {
                *self.progress.write().await? = Clone_progress::Complete;
                Ok(((), task::Status::Built))
            }
            Err(error) => {
                *self.progress
                    .write()
                    .await? = Clone_progress::Failed(format!("{error:#}"));
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
