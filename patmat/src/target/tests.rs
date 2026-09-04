use super::*;
use crate::task::{Status, TaskResult, TaskTrait};

struct EmptyTask;

#[async_trait]
impl TaskTrait for EmptyTask {
    type Output = ();

    async fn run(&self, _widget: Store<Option<Widget>>) -> TaskResult<Self::Output> {
        Ok(((), Status::Built))
    }
}

struct PathTask(PathBuf);

#[async_trait]
impl TaskTrait for PathTask {
    type Output = ();

    async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        path.set(Some(self.0.clone())).await?;
        Ok(())
    }

    async fn run(&self, _widget: Store<Option<Widget>>) -> TaskResult<Self::Output> {
        Ok(((), Status::Built))
    }
}

#[tokio::test]
async fn metadata_clones_share_each_store() -> Result<()> {
    let target = Target::new_independent("before", Task::new(EmptyTask));
    let metadata = target.get_metadata();

    metadata.name.set("after".to_owned()).await?;
    metadata.path.set(Some(PathBuf::from("updated"))).await?;

    let current = target.get_metadata();
    assert_eq!(&*current.name.read().await?, "after");
    assert_eq!(
        current.path.read().await?.as_deref(),
        Some(std::path::Path::new("updated"))
    );
    Ok(())
}

#[tokio::test]
async fn task_init_sets_target_path() -> Result<()> {
    let target = Target::new_independent(
        "with_path",
        Task::new(PathTask(PathBuf::from("/test/path"))),
    );
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let metadata = target.get_metadata();
    assert_eq!(
        metadata.path.read().await?.as_deref(),
        Some(std::path::Path::new("/test/path"))
    );
    Ok(())
}
