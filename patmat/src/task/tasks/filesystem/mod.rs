mod task;

use crate::task::Task;
use std::path::PathBuf;

pub fn copy_file(source: PathBuf, destination: PathBuf) -> Task<()> {
    Task::new(task::Copy_file_task {
        source,
        destination,
    })
}

pub fn copy_dir(source: PathBuf, destination: PathBuf) -> Task<()> {
    Task::new(task::Copy_dir_task {
        source,
        destination,
    })
}

pub fn create_dir(path: PathBuf) -> Task<()> {
    Task::new(task::Create_dir_task { path })
}

pub fn write_file(path: PathBuf, content: String) -> Task<()> {
    Task::new(task::Write_file_task { path, content })
}

pub fn create_directory(path: PathBuf, subdirs: Vec<String>) -> Task<PathBuf> {
    Task::new(task::Create_directory_task { path, subdirs })
}
