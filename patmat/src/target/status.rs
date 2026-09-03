use std::sync::Arc;

use color_eyre::eyre::Error;
use lucide_icons::Icon;

use crate::task::Status;

#[derive(Clone)]
pub enum TargetStatus {
    Unsatisfied,
    Running,
    // Error is gonna be read only so an Arc is fine
    Error(Arc<Error>),
    Satisfied(Status),
    RunningDependencies,
}

impl TargetStatus {
    pub fn label(&self) -> &'static str {
        match self {
            TargetStatus::Unsatisfied => "Unsatisfied",
            TargetStatus::RunningDependencies => "Running dependencies",
            TargetStatus::Running => "Running",
            TargetStatus::Satisfied(Status::Built) => "Built",
            TargetStatus::Satisfied(Status::AlreadyBuilt) => "Already built",
            TargetStatus::Error(_) => "Error",
        }
    }

    pub fn get_icon(&self) -> Icon {
        match self {
            TargetStatus::Unsatisfied => Icon::CircleDashed,
            TargetStatus::RunningDependencies => Icon::CircleEllipsis,
            TargetStatus::Running => Icon::Loader,
            TargetStatus::Satisfied(status) => match status {
                Status::Built => Icon::Hammer,
                Status::AlreadyBuilt => Icon::CheckCircle,
            },
            TargetStatus::Error(_err) => Icon::CircleX,
        }
    }

    pub fn satisfied(&self) -> bool {
        matches!(self, TargetStatus::Satisfied(_))
    }
}
