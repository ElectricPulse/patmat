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

use crate::{target::status::TargetStatus, task::Task};
use color_eyre::eyre::{Result, eyre};
use drevo::{
    state::Store,
    sync::{Mutex, ThreadSafe},
    widget::Widget,
};

pub trait OutputConstraints: ThreadSafe + Clone {}
impl<T> OutputConstraints for T where T: ThreadSafe + Clone {}

static NEXT_TARGET_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct TargetMetadata {
    pub id: Store<u64>,
    pub name: Store<String>,
    /// This path is only used to show the user roughly where the task is working.
    pub path: Store<Option<PathBuf>>,
    pub dependencies: Store<Dependencies>,
    pub status: Store<TargetStatus>,
}

#[async_trait]
// Since Targets are clonable and aren't in an Arc, Target trait should also be clonable
pub trait TargetTrait: DynClone + Send + Sync {
    fn get_metadata(&self) -> TargetMetadata;
    fn widget(&self) -> Store<Option<Widget>>;
    async fn ensure_ran(&self) -> Result<()>;
}

dyn_clone::clone_trait_object!(TargetTrait);

pub type Dependency = Box<dyn TargetTrait>;
pub type Dependencies = Vec<Dependency>;

// Target should not be returned from helper functions
// only the task
#[derive(Clone)]
pub struct Target<Output: OutputConstraints> {
    metadata: TargetMetadata,
    task: Task<Output>,
    widget: Store<Option<Widget>>,
    output: Arc<Mutex<Option<Output>>>,
}

impl<Output: OutputConstraints> Target<Output> {
    pub fn get_metadata(&self) -> TargetMetadata {
        self.metadata.clone()
    }

    pub fn widget(&self) -> Store<Option<Widget>> {
        self.widget.clone()
    }

    pub fn new_independent(name: impl Into<String>, task: Task<Output>) -> Self {
        Self::new(name, task, Dependencies::new())
    }

    pub fn new(name: impl Into<String>, task: Task<Output>, dependencies: Dependencies) -> Self {
        let metadata = TargetMetadata {
            id: Store::new(NEXT_TARGET_ID.fetch_add(1, Ordering::Relaxed)),
            name: Store::new(name.into()),
            path: Store::new(None),
            dependencies: Store::new(dependencies),
            status: Store::new(TargetStatus::Unsatisfied),
        };

        let path_store = metadata.path.clone();
        let task_clone = task.clone();
        let _ = tokio::spawn(async move {
            let _ = task_clone.init(path_store).await;
        });

        Self {
            metadata,
            task,
            widget: Store::new(None),
            output: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get(&self) -> Result<Output> {
        let mut output_guard = self.output.lock().await?;
        if let Some(output) = &*output_guard {
            return Ok(output.clone());
        }

        self.set_status(TargetStatus::RunningDependencies).await?;

        let dependencies = self.metadata.dependencies.read().await?.clone();

        for dependency in dependencies {
            dependency.ensure_ran().await?;
        }

        self.set_status(TargetStatus::Running).await?;

        let result = self.task.task.lock().await?.run(self.widget.clone()).await;

        let (output, status) = match result {
            Err(err) => {
                let error_message = format!("{err:#}");
                self.set_status(TargetStatus::Error(Arc::new(err))).await?;
                return Err(eyre!("{error_message}"));
            }
            Ok(result) => result,
        };

        self.set_status(TargetStatus::Satisfied(status)).await?;
        *output_guard = Some(output.clone());

        Ok(output)
    }

    async fn set_status(&self, status: TargetStatus) -> Result<()> {
        self.metadata.status.set(status).await?;
        Ok(())
    }
}

impl<Output: OutputConstraints> From<Target<Output>> for Dependency {
    fn from(target: Target<Output>) -> Self {
        Box::new(target)
    }
}

#[async_trait]
impl<Output: OutputConstraints> TargetTrait for Target<Output> {
    fn get_metadata(&self) -> TargetMetadata {
        self.metadata.clone()
    }

    fn widget(&self) -> Store<Option<Widget>> {
        self.widget.clone()
    }

    async fn ensure_ran(&self) -> Result<()> {
        let _ = self.get().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
