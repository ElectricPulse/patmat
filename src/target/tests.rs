use super::*;
use crate::task::{Status, Task_result, Task_trait};

struct Empty_task;

#[async_trait]
impl Task_trait for Empty_task {
    type Output = ();

    async fn run(&self, _widget: Store<Option<Widget>>) -> Task_result<Self::Output> {
        Ok(((), Status::Built))
    }
}

#[tokio::test]
async fn metadata_clones_share_each_store() -> Result<()> {
    let target = Target::new_independent("before", None, Task::new(Empty_task));
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
