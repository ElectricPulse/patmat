mod task;

use crate::target::{Dependencies, Target};
use std::path::PathBuf;

pub fn new(
    name: impl Into<String>,
    repo_path: PathBuf,
    branch: String,
    dependencies: Dependencies,
) -> Target<()> {
    let task = task::Task::new(repo_path, branch.clone());
    let path = task.repo_path.clone();
    Target::new(name, Some(path), task, dependencies)
}
