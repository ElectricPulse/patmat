use crate::task::{self, Status, Task};

use async_trait::async_trait;

use std::sync::Arc;
use vizual::{sync::Mutex, widget::Widget};

#[derive(Clone, Copy)]
struct Require_task {}

#[async_trait]
impl task::Task_trait for Require_task {
    type Output = ();
    async fn run(&self, _widget: Arc<Mutex<Option<Widget>>>) -> task::Task_result {
        Ok(((), Status::Built))
    }
}

pub fn new() -> Task<()> {
    Task::new(Require_task {})
}
