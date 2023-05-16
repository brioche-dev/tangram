use thiserror::Error;

/// An error from a process.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
pub enum Error {
	#[error(r#"The process exited with status code {0}."#)]
	Code(i32),

	#[error(r#"The process exited with status signal {0}."#)]
	Signal(i32),
}
