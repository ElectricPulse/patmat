use crate::task;

use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use drevo::{state::Store, widget::Widget};

#[derive(Clone)]
pub(super) struct CopyFileTask {
    pub(super) source: PathBuf,
    pub(super) destination: PathBuf,
}

#[async_trait]
impl task::TaskTrait for CopyFileTask {
    type Output = ();

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        path.set(Some(self.destination.clone())).await?;
        Ok(())
    }

    async fn run(&self, _widget: Store<Option<Widget>>) -> task::TaskResult {
        let _ = tokio::fs::copy(&self.source, &self.destination)
            .await
            .wrap_err(format!(
                "Failed to copy file from {:?} to {:?}",
                self.source, self.destination
            ))?;

        Ok(((), task::Status::Built))
    }
}

#[derive(Clone)]
pub(super) struct CopyDirTask {
    pub(super) source: PathBuf,
    pub(super) destination: PathBuf,
}

#[async_trait]
impl task::TaskTrait for CopyDirTask {
    type Output = ();

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        path.set(Some(self.destination.clone())).await?;
        Ok(())
    }

    async fn run(&self, _widget: Store<Option<Widget>>) -> task::TaskResult {
        // Check if dest already exists
        if self.destination.exists()
            && let Some(content) = self.destination.read_dir()?.next()
        {
            let _ = content?;
            return Ok(((), task::Status::AlreadyBuilt));
        }

        copy_dir_all(&self.source, &self.destination).wrap_err(format!(
            "Failed to copy directory from {:?} to {:?}",
            self.source, self.destination
        ))?;

        Ok(((), task::Status::Built))
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            let _ = fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[derive(Clone)]
pub(super) struct CreateDirTask {
    pub(super) path: PathBuf,
}

#[async_trait]
impl task::TaskTrait for CreateDirTask {
    type Output = ();

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        path.set(Some(self.path.clone())).await?;
        Ok(())
    }

    async fn run(&self, _widget: Store<Option<Widget>>) -> task::TaskResult {
        if self.path.exists() {
            return Ok(((), task::Status::AlreadyBuilt));
        }

        tokio::fs::create_dir_all(&self.path)
            .await
            .wrap_err(format!("Failed to create directory: {:?}", self.path))?;

        Ok(((), task::Status::Built))
    }
}

#[derive(Clone)]
pub(super) struct WriteFileTask {
    pub(super) path: PathBuf,
    pub(super) content: String,
}

#[async_trait]
impl task::TaskTrait for WriteFileTask {
    type Output = ();

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        path.set(Some(self.path.clone())).await?;
        Ok(())
    }

    async fn run(&self, _widget: Store<Option<Widget>>) -> task::TaskResult {
        // Check if file exists with same content
        if self.path.exists()
            && let Ok(existing_content) = tokio::fs::read_to_string(&self.path).await
            && existing_content == self.content
        {
            return Ok(((), task::Status::AlreadyBuilt));
        }

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .wrap_err(format!("Failed to create parent directory: {:?}", parent))?;
        }

        tokio::fs::write(&self.path, &self.content)
            .await
            .wrap_err(format!("Failed to write file: {:?}", self.path))?;

        Ok(((), task::Status::Built))
    }
}

#[derive(Clone)]
pub(super) struct CreateDirectoryTask {
    pub(super) path: PathBuf,
    pub(super) subdirs: Vec<String>,
}

#[async_trait]
impl task::TaskTrait for CreateDirectoryTask {
    type Output = PathBuf;

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        path.set(Some(self.path.clone())).await?;
        Ok(())
    }

    async fn run(&self, _widget: Store<Option<Widget>>) -> task::TaskResult<PathBuf> {
        if self.path.exists() {
            return Ok((self.path.clone(), task::Status::AlreadyBuilt));
        }

        tokio::fs::create_dir_all(&self.path)
            .await
            .wrap_err(format!("Failed to create directory: {:?}", self.path))?;

        // Create subdirectories if specified
        for subdir in &self.subdirs {
            let subdir_path = self.path.join(subdir);
            tokio::fs::create_dir_all(&subdir_path)
                .await
                .wrap_err(format!("Failed to create subdirectory: {:?}", subdir_path))?;
        }

        Ok((self.path.clone(), task::Status::Built))
    }
}
