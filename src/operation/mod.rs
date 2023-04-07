pub use self::{
	data::Data,
	error::{Error, Result},
	hash::Hash,
};
pub use crate::{call::Call, download::Download, process::Process};

mod children;
mod data;
mod error;
mod get;
mod hash;
mod output;
mod run;

/// An operation.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Operation {
	/// A call operation.
	Call(Call),

	/// A download operation.
	Download(Download),

	/// A process operation.
	Process(Process),
}

impl Operation {
	#[must_use]
	pub fn hash(&self) -> Hash {
		match self {
			Self::Call(call) => call.hash(),
			Self::Process(process) => process.hash(),
			Self::Download(download) => download.hash(),
		}
	}
}

impl From<Call> for Operation {
	fn from(value: Call) -> Self {
		Self::Call(value)
	}
}

impl From<Download> for Operation {
	fn from(value: Download) -> Self {
		Self::Download(value)
	}
}

impl From<Process> for Operation {
	fn from(value: Process) -> Self {
		Self::Process(value)
	}
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
