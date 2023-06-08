use thiserror::Error;

/// An error from a command.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
pub enum Error {
	#[error(r#"The process exited with code {0}."#)]
	Code(i32),

	#[error(r#"The process exited with signal {0}."#)]
	Signal(i32),
}
