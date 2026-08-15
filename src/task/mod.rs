pub mod tasks;

use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
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
    async fn run(&self) -> Task_result<Self::Output>;
}

#[derive(Clone)]
pub struct Task<Output = ()> {
    pub widget: Option<Widget>,
    pub(crate) task: Arc<Mutex<dyn Task_trait<Output = Output>>>,
}

impl<Output> Task<Output> {
    pub fn new(task: impl Task_trait<Output = Output> + 'static, widget: Option<Widget>) -> Self {
        Self {
            widget,
            task: Arc::new(Mutex::new(task)),
        }
    }
}
