pub mod status;
pub mod targets;
pub mod task;

use async_trait::async_trait;
use dyn_clone::DynClone;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::target::{status::Target_status, task::Task_trait};
use color_eyre::eyre::{Result, eyre};
use vizual::{
    state::{State, Store},
    sync::{Mutex, Thread_safe},
    widget::{Widget, Widget_trait},
};

pub trait Output_constraints: Thread_safe + Clone {}
impl<T> Output_constraints for T where T: Thread_safe + Clone {}

type Task<Output> = Box<dyn Task_trait<Output = Output>>;

static NEXT_TARGET_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Target_metadata {
    pub(crate) id: Store<u64>,
    pub(crate) name: Store<String>,
    /// This path is only used to show the user roughly where the task is working.
    pub(crate) path: Store<Option<PathBuf>>,
    pub(crate) dependencies: Store<Dependencies>,
    pub(crate) status: Store<Target_status>,
}

struct Task_manager<Output: Output_constraints> {
    task: Task<Output>,
    output: Option<Output>,
    metadata: Target_metadata,
}

// Note that Task_manager get() will now block if a task is in progress
// We don't close the task even though we don't need it during dependency execution
// because we want any one else waiting for .get() to have to wait for the output not start executing dependencies or the task again
impl<Output: Output_constraints> Task_manager<Output> {
    async fn get(&mut self) -> Result<Output> {
        if let Some(output) = &self.output {
            return Ok(output.clone());
        }

        self.set_status(Target_status::Running_dependencies).await?;

        let dependencies = self.metadata.dependencies.read().await?.clone();

        for dependency in dependencies {
            dependency.ensure_ran().await?;
        }

        self.set_status(Target_status::Running).await?;

        let result = self.task.run().await;

        let (output, status) = match result {
            Err(err) => {
                self.set_status(Target_status::Error(Arc::new(err))).await?;
                return Err(eyre!("Task failed"));
            }
            Ok(result) => result,
        };

        self.set_status(Target_status::Satisfied(status)).await?;
        self.output = Some(output.clone());

        Ok(output)
    }

    async fn set_status(&self, status: Target_status) -> Result<()> {
        *self.metadata.status.write().await? = status;
        Ok(())
    }
}

#[async_trait]
// Since Targets are clonable and aren't in an Arc, Target trait should also be clonable
pub trait Target_trait: DynClone + Send + Sync {
    fn get_metadata(&self) -> Target_metadata;
    async fn ensure_ran(&self) -> Result<()>;
    fn widget(&self) -> Option<Widget>;
}

dyn_clone::clone_trait_object!(Target_trait);

// A quick word on how dependencies are added into a Target.
// Dependencies right now are passed via arguments into Target::new()
// it would make more sense to not do that and just make it so that targets are first created as Target_definition
// that can then be converted into a Target -> after adding dependcies the user wants.
// This would add extra complexity and add two new methods add_dependency and some Into<>
// An alternative is to implement add_dependency as an async on the Target - locking dependencies and appending to that vector
// but that is quite unclean aswell.
// Until the issue crystalizes the current architecture looks the cleanest.
pub type Dependency = Box<dyn Target_trait>;
pub type Dependencies = Vec<Dependency>;

// A target is sharable and clonable by default
// This is done because it simplifies the architecture.
// Also a seperate non sharable target would at the cost of a lot of added complexity only offer a small performance benefit
// of removing shared state that only really gets accessed in one place.
// Task_manager remains separately locked while each metadata field is independently accessible
// through its Store, even while a task is running.
// Task_manager bundles output and task because there is no reason to try fetching the output when a task is in progress -
// one has to wait till it's done seperately
// The task manager carries cloned Store handles so it can update status without owning the UI's
// Target_metadata value.
// Widget is a separate field because it must remain renderable while the task is locked during a
// build. A widget that needs shared mutable state can carry that state itself.
#[derive(Clone)]
pub struct Target<Output: Output_constraints> {
    metadata: Target_metadata,
    task: Arc<Mutex<Task_manager<Output>>>,
    // The widget is intentionally separate from the task. The task manager stays locked for the
    // entire build, so a widget stored as the task itself could not be rendered while that build
    // was running. Keeping presentation separate also lets the same task implementation be used
    // by different targets with different UIs.
    widget: Option<Widget>,
}

impl<Output: Output_constraints> Target<Output> {
    pub fn new_independent(
        name: impl Into<String>,
        path: Option<PathBuf>,
        task: impl Task_trait<Output = Output> + 'static,
    ) -> Self {
        Self::new(name, path, task, Dependencies::new())
    }

    pub fn new(
        name: impl Into<String>,
        path: Option<PathBuf>,
        task: impl Task_trait<Output = Output> + 'static,
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
            widget: None,
            metadata: metadata.clone(),
            task: Arc::new(Mutex::new(Task_manager {
                output: None,
                task: Box::new(task),
                metadata,
            })),
        }
    }

    pub fn set_widget(&mut self, widget: impl Widget_trait) {
        self.widget = Some(Box::new(widget));
    }

    pub async fn get(&self) -> Result<Output> {
        self.task.lock().await?.get().await
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

    fn widget(&self) -> Option<Widget> {
        self.widget.clone()
    }

    async fn ensure_ran(&self) -> Result<()> {
        let _ = self.get().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::task::{Status, Task_result};

    struct Empty_task;

    #[async_trait]
    impl Task_trait for Empty_task {
        type Output = ();

        async fn run(&self) -> Task_result<Self::Output> {
            Ok(((), Status::Built))
        }
    }

    #[tokio::test]
    async fn metadata_clones_share_each_store() -> Result<()> {
        let target = Target::new_independent("before", None, Empty_task);
        let metadata = target.get_metadata();

        *metadata.name.write().await? = "after".to_owned();
        *metadata.path.write().await? = Some(PathBuf::from("updated"));

        let current = target.get_metadata();
        assert_eq!(&*current.name.read().await?, "after");
        assert_eq!(
            current.path.read().await?.as_deref(),
            Some(std::path::Path::new("updated"))
        );
        Ok(())
    }
}
