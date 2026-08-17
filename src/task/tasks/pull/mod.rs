use crate::task::Task;
use crate::task::tasks::terminal;
use std::path::PathBuf;

pub fn new(repo_path: PathBuf) -> Task<()> {
    terminal::new_in_dir("git pull", repo_path)
}
