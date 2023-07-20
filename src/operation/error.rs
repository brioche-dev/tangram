use crate::{resource, target, task};
use thiserror::Error;

/// An operation result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An operation error.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Error {
	/// An error from a resource.
	#[error(transparent)]
	Resource(#[from] resource::Error),

	/// An error from a target.
	#[error(transparent)]
	Target(#[from] target::Error),

	/// An error from a task.
	#[error(transparent)]
	Task(#[from] task::Error),

	/// A cancellation.
	#[error("The operation was cancelled.")]
	Cancellation,
}
