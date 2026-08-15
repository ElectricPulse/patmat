mod progress;
mod task;

use crate::task::Task;
use std::path::PathBuf;

pub fn new(path: PathBuf, remote_path: String) -> Task<()> {
    let task = task::Task::new(path, remote_path);
    let widget = task.widget();
    Task::new(task, Some(Box::new(widget)))
}
