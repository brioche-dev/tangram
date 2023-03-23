use thiserror::Error;

/// An error from a process.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[error(r#"The process exited with code "{code}"."#)]
pub struct Error {
	code: i32,
}
