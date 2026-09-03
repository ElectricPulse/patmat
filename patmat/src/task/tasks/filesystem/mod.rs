mod task;

use crate::task::Task;
use std::path::PathBuf;

pub fn copy_file(source: PathBuf, destination: PathBuf) -> Task<()> {
    Task::new(task::CopyFileTask {
        source,
        destination,
    })
}

pub fn copy_dir(source: PathBuf, destination: PathBuf) -> Task<()> {
    Task::new(task::CopyDirTask {
        source,
        destination,
    })
}

pub fn create_dir(path: PathBuf) -> Task<()> {
    Task::new(task::CreateDirTask { path })
}

pub fn write_file(path: PathBuf, content: String) -> Task<()> {
    Task::new(task::WriteFileTask { path, content })
}

pub fn create_directory(path: PathBuf, subdirs: Vec<String>) -> Task<PathBuf> {
    Task::new(task::CreateDirectoryTask { path, subdirs })
}
