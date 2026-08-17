mod progress;
mod task;

use std::path::PathBuf;
use vizual::widget::{Widget_trait, widgets::positioning::anchor::Anchor};

use crate::task::Task;

pub fn new(path: PathBuf, remote_path: String) -> Task<()> {
    let task = task::Task::new(path, remote_path);
    let widget = Anchor::middle(task.widget());
    Task::new(task, Some(widget.any()))
}
