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

struct Closure_task<F, Output> {
    func: F,
    _marker: std::marker::PhantomData<fn() -> Output>,
}

#[async_trait]
impl<F, Fut, Output: Send + Sync + 'static> Task_trait for Closure_task<F, Output>
where
    F: Fn(Store<Option<Widget>>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Task_result<Output>> + Send + 'static,
{
    type Output = Output;

    async fn run(&self, widget: Store<Option<Widget>>) -> Task_result<Self::Output> {
        (self.func)(widget).await
    }
}

impl<Output: Send + Sync + 'static> Task<Output> {
    pub fn new(task: impl Task_trait<Output = Output> + 'static) -> Self {
        Self {
            task: Arc::new(Mutex::new(task)),
        }
    }

    pub fn from_fn<F, Fut>(f: F) -> Self
    where
        F: Fn(Store<Option<Widget>>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Task_result<Output>> + Send + 'static,
    {
        Self::new(Closure_task {
            func: f,
            _marker: std::marker::PhantomData,
        })
    }

    pub async fn init(&self, path: Store<Option<PathBuf>>) -> Result<()> {
        self.task.lock().await?.init(path).await
    }
}

impl Task<()> {
    pub fn from_run<F, Fut>(f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        Self::from_fn(move |_| {
            let fut = f();
            async move {
                fut.await?;
                Ok(((), Status::Built))
            }
        })
    }
}
