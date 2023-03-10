pub use self::hash::Hash;
pub use crate::{call::Call, download::Download, process::Process};

mod add;
mod children;
mod get;
mod hash;
mod output;
mod run;
mod serialize;

/// An operation.
#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Operation {
	/// A download operation.
	#[buffalo(id = 0)]
	#[serde(rename = "download")]
	Download(Download),

	/// A process operation.
	#[buffalo(id = 1)]
	#[serde(rename = "process")]
	Process(Process),

	/// A call operation.
	#[buffalo(id = 2)]
	#[serde(rename = "call")]
	Call(Call),
}

impl Operation {
	#[must_use]
	pub fn as_download(&self) -> Option<&Download> {
		if let Operation::Download(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_process(&self) -> Option<&Process> {
		if let Operation::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_call(&self) -> Option<&Call> {
		if let Operation::Call(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Operation {
	#[must_use]
	pub fn into_download(self) -> Option<Download> {
		if let Operation::Download(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_process(self) -> Option<Process> {
		if let Operation::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_call(self) -> Option<Call> {
		if let Operation::Call(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
