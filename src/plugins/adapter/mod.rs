pub mod generic;
pub mod traits;

pub use generic::GenericAdapter;
pub use traits::{HostAction, WorkflowAdapter};

pub use crate::workflows::engine::Suggestion;
