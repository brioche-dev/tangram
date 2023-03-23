use crate::{call, download, process};
use thiserror::Error;

/// An operation result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An operation error.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Error {
	/// An error from a download.
	#[error(transparent)]
	#[serde(rename = "download")]
	Download(#[from] download::Error),

	/// An error from a process.
	#[error(transparent)]
	#[serde(rename = "process")]
	Process(#[from] process::Error),

	/// An error from a call.
	#[error(transparent)]
	#[serde(rename = "call")]
	Call(#[from] call::Error),

	/// A cancellation.
	#[error("The operation was cancelled.")]
	#[serde(rename = "cancellation")]
	Cancellation,
}
