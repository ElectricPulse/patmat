mod task;

use crate::task::Task;
use std::path::PathBuf;

pub fn new(repo_path: PathBuf, branch: String) -> Task<()> {
    Task::new(task::Task::new(repo_path, branch), None)
}
