pub mod status;

use async_trait::async_trait;
use dyn_clone::DynClone;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{target::status::Target_status, task::Task};
use color_eyre::eyre::{Result, eyre};
use vizual::{
    state::{State, Store},
    sync::{Mutex, Thread_safe},
    widget::Widget,
};

pub trait Output_constraints: Thread_safe + Clone {}
impl<T> Output_constraints for T where T: Thread_safe + Clone {}

static NEXT_TARGET_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Target_metadata {
    pub id: Store<u64>,
    pub name: Store<String>,
    /// This path is only used to show the user roughly where the task is working.
    pub path: Store<Option<PathBuf>>,
    pub dependencies: Store<Dependencies>,
    pub status: Store<Target_status>,
}

#[async_trait]
// Since Targets are clonable and aren't in an Arc, Target trait should also be clonable
pub trait Target_trait: DynClone + Send + Sync {
    fn get_metadata(&self) -> Target_metadata;
    fn widget(&self) -> Arc<Mutex<Option<Widget>>>;
    async fn ensure_ran(&self) -> Result<()>;
}

dyn_clone::clone_trait_object!(Target_trait);

pub type Dependency = Box<dyn Target_trait>;
pub type Dependencies = Vec<Dependency>;

#[derive(Clone)]
pub struct Target<Output: Output_constraints> {
    metadata: Target_metadata,
    task: Task<Output>,
    widget: Arc<Mutex<Option<Widget>>>,
    output: Arc<Mutex<Option<Output>>>,
}

impl<Output: Output_constraints> Target<Output> {
    pub fn get_metadata(&self) -> Target_metadata {
        self.metadata.clone()
    }

    pub fn widget(&self) -> Arc<Mutex<Option<Widget>>> {
        self.widget.clone()
    }

    pub fn new_independent(
        name: impl Into<String>,
        path: Option<PathBuf>,
        task: Task<Output>,
    ) -> Self {
        Self::new(name, path, task, Dependencies::new())
    }

    pub fn new(
        name: impl Into<String>,
        path: Option<PathBuf>,
        task: Task<Output>,
        dependencies: Dependencies,
    ) -> Self {
        let metadata = Target_metadata {
            id: Store::new(NEXT_TARGET_ID.fetch_add(1, Ordering::Relaxed)),
            name: Store::new(name.into()),
            path: Store::new(path),
            dependencies: Store::new(dependencies),
            status: Store::new(Target_status::Unsatisfied),
        };

        Self {
            metadata,
            task,
            widget: Arc::new(Mutex::new(None)),
            output: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get(&self) -> Result<Output> {
        let mut output_guard = self.output.lock().await?;
        if let Some(output) = &*output_guard {
            return Ok(output.clone());
        }

        self.set_status(Target_status::Running_dependencies).await?;

        let dependencies = self.metadata.dependencies.read().await?.clone();

        for dependency in dependencies {
            dependency.ensure_ran().await?;
        }

        self.set_status(Target_status::Running).await?;

        let result = self
            .task
            .task
            .lock()
            .await?
            .run(self.widget.clone())
            .await;

        let (output, status) = match result {
            Err(err) => {
                self.set_status(Target_status::Error(Arc::new(err))).await?;
                return Err(eyre!("Task failed"));
            }
            Ok(result) => result,
        };

        self.set_status(Target_status::Satisfied(status)).await?;
        *output_guard = Some(output.clone());

        Ok(output)
    }

    async fn set_status(&self, status: Target_status) -> Result<()> {
        *self.metadata.status.write().await? = status;
        Ok(())
    }
}

impl<Output: Output_constraints> From<Target<Output>> for Dependency {
    fn from(target: Target<Output>) -> Self {
        Box::new(target)
    }
}

#[async_trait]
impl<Output: Output_constraints> Target_trait for Target<Output> {
    fn get_metadata(&self) -> Target_metadata {
        self.metadata.clone()
    }

    fn widget(&self) -> Arc<Mutex<Option<Widget>>> {
        self.widget.clone()
    }

    async fn ensure_ran(&self) -> Result<()> {
        let _ = self.get().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
