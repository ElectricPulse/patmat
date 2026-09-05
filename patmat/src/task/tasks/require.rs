use crate::task::{self, Status, Task};

use async_trait::async_trait;

use drevo::{state::Store, widget::Widget};

#[derive(Clone, Copy)]
struct RequireTask {}

#[async_trait]
impl task::TaskTrait for RequireTask {
    type Output = ();
    async fn run(&self, _widget: Store<Option<Widget>>) -> task::TaskResult {
        Ok(((), Status::Built))
    }
}

pub fn new() -> Task<()> {
    Task::new(RequireTask {})
}
