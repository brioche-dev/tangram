use crate::operation;
use thiserror::Error;

#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[error(transparent)]
pub struct Error {
	source: Box<operation::Error>,
}
