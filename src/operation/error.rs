use crate::{command, function, resource};
use thiserror::Error;

/// An operation result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An operation error.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Error {
	/// An error from a command.
	#[error(transparent)]
	Command(#[from] command::Error),

	/// An error from a function.
	#[error(transparent)]
	Function(#[from] function::Error),

	/// An error from a resource.
	#[error(transparent)]
	Resource(#[from] resource::Error),

	/// A cancellation.
	#[error("The operation was cancelled.")]
	Cancellation,
}
