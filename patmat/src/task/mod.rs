pub mod tasks;

use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    state::Store,
    sync::Mutex,
    widget::Widget,
};

#[derive(Clone)]
pub enum Status {
    Built,
    Already_built,
}

pub type Task_result<Output = ()> = Result<(Output, Status)>;

#[async_trait]
pub trait Task_trait: Send + Sync {
    type Output: Send;
    async fn init(&self, _path: Store<Option<PathBuf>>) -> Result<()> {
        Ok(())
    }
    async fn run(&self, widget: Store<Option<Widget>>) -> Task_result<Self::Output>;
}

#[derive(Clone)]
pub struct Task<Output: Send = ()> {
    pub(crate) task: Arc<Mutex<dyn Task_trait<Output = Output>>>,
}

impl<Output: Send> Task<Output> {
    pub fn new(task: impl Task_trait<Output = Output> + 'static) -> Self {
        Self {
            task: Arc::new(Mutex::new(task)),
        }
    }

    pub async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        self.task.lock().await?.init(path).await
    }
}
