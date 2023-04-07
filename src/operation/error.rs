use crate::{call, download, process};
use thiserror::Error;

/// An operation result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An operation error.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Error {
	/// An error from a download.
	#[error(transparent)]
	Download(#[from] download::Error),

	/// An error from a process.
	#[error(transparent)]
	Process(#[from] process::Error),

	/// An error from a call.
	#[error(transparent)]
	Call(#[from] call::Error),

	/// A cancellation.
	#[error("The operation was cancelled.")]
	Cancellation,
}
