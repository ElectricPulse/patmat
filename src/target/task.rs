use async_trait::async_trait;

use color_eyre::eyre::Result;

#[derive(Clone)]
pub enum Status {
    Built,
    Already_built,
}

#[async_trait]
pub trait Task_trait: Send + Sync {
    type Output: Send;
    // In the future some check() should get seperated from build().
    // Leaving place for the target to have two types of dependencies one set for check() -> for example a database connection.
    // And one set of dependencies for build() -> the information to rebuild the database record if during check it notices that they dont exist.
    // There could be a third set of dependencies called optional. These would be .get() deps that during build get conditionally required
    async fn run(&self) -> Task_result<Self::Output>;
}

pub type Task_result<Output = ()> = Result<(Output, Status)>;
