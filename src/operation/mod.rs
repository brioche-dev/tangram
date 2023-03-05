pub use self::hash::Hash;
use crate::Instance;
pub use crate::{call::Call, download::Download, process::Process};
use anyhow::{bail, Context, Result};

mod children;
mod hash;
mod output;
mod run;
mod serialize;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Operation {
	#[buffalo(id = 0)]
	#[serde(rename = "download")]
	Download(Download),

	#[buffalo(id = 1)]
	#[serde(rename = "process")]
	Process(Process),

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

impl Instance {
	pub fn get_operation_local(&self, hash: Hash) -> Result<Operation> {
		let operation = self
			.try_get_operation_local(hash)?
			.with_context(|| format!(r#"Failed to find the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub fn get_operation_local_with_txn<Txn>(&self, txn: &Txn, hash: Hash) -> Result<Operation>
	where
		Txn: lmdb::Transaction,
	{
		let operation = self
			.try_get_operation_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the operation with hash "{hash}"."#))?;
		Ok(operation)
	}

	pub fn try_get_operation_local(&self, hash: Hash) -> Result<Option<Operation>> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		// Get the operation.
		let maybe_operation = self.try_get_operation_local_with_txn(&txn, hash)?;

		Ok(maybe_operation)
	}

	/// Try to get an operation from the database with the given transaction.
	pub fn try_get_operation_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Option<Operation>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.database.operations, &hash.as_slice()) {
			Ok(value) => {
				let value = buffalo::from_slice(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}
