mod progress;
mod task;

use std::path::PathBuf;

use crate::task::Task;

pub fn new(path: PathBuf, remote_path: String) -> Task<()> {
    Task::new(task::Task::new(path, remote_path))
}
