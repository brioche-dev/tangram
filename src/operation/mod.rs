pub use self::{download::Download, hash::OperationHash, process::Process, target::Target};
use crate::Cli;
use anyhow::{bail, Context, Result};

mod children;
mod download;
mod hash;
mod output;
mod process;
mod run;
mod serialize;
mod target;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(tag = "type", content = "value")]
pub enum Operation {
	#[buffalo(id = 0)]
	#[serde(rename = "download")]
	Download(Download),

	#[buffalo(id = 1)]
	#[serde(rename = "process")]
	Process(Process),

	#[buffalo(id = 2)]
	#[serde(rename = "target")]
	Target(Target),
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
	pub fn as_target(&self) -> Option<&Target> {
		if let Operation::Target(v) = self {
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
	pub fn into_target(self) -> Option<Target> {
		if let Operation::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Cli {
	pub fn get_operation_local(&self, hash: OperationHash) -> Result<Operation> {
		let operation = self
			.try_get_operation_local(hash)?
			.with_context(|| format!(r#"Failed to find the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub fn get_operation_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: OperationHash,
	) -> Result<Operation>
	where
		Txn: lmdb::Transaction,
	{
		let operation = self
			.try_get_operation_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub fn try_get_operation_local(&self, hash: OperationHash) -> Result<Option<Operation>> {
		// Begin a read transaction.
		let txn = self.inner.database.env.begin_ro_txn()?;

		// Get the operation.
		let maybe_operation = self.try_get_operation_local_with_txn(&txn, hash)?;

		Ok(maybe_operation)
	}

	/// Try to get an operation from the database with the given transaction.
	pub fn try_get_operation_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: OperationHash,
	) -> Result<Option<Operation>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.inner.database.operations, &hash.as_slice()) {
			Ok(value) => {
				let value = buffalo::from_slice(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}
