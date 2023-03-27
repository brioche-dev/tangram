use crate::{language, operation};
use std::sync::Arc;
use thiserror::Error;

/// A result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Error {
	/// An error with a message.
	#[error("{message}\n  {location}")]
	#[serde(rename = "message")]
	Message {
		message: String,
		location: Location,
		source: Option<Arc<Error>>,
	},

	/// An operation error.
	#[error(transparent)]
	#[serde(rename = "operation")]
	Operation(#[from] operation::Error),

	// /// A language service error.
	// #[error(transparent)]
	// #[serde(rename = "language_service")]
	// LanguageService(#[from] language::service::error::Error),
	//
	/// Any other error.
	#[error("{message}")]
	#[serde(rename = "other")]
	Other {
		message: String,
		source: Option<Arc<Error>>,
	},
}

impl Error {
	#[track_caller]
	pub fn message(message: impl std::fmt::Display) -> Error {
		Error::Message {
			message: message.to_string(),
			location: Location::caller(),
			source: None,
		}
	}

	pub fn other(error: impl std::error::Error) -> Error {
		Error::Other {
			message: error.to_string(),
			source: error.source().map(|error| Arc::new(Error::other(error))),
		}
	}
}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Error {
		Error::Other {
			message: error.to_string(),
			source: std::error::Error::source(&error).map(|error| Arc::new(Error::other(error))),
		}
	}
}

impl From<lmdb::Error> for Error {
	fn from(error: lmdb::Error) -> Error {
		Error::Other {
			message: error.to_string(),
			source: std::error::Error::source(&error).map(|error| Arc::new(Error::other(error))),
		}
	}
}

impl From<reqwest::Error> for Error {
	fn from(error: reqwest::Error) -> Error {
		Error::Other {
			message: error.to_string(),
			source: std::error::Error::source(&error).map(|error| Arc::new(Error::other(error))),
		}
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Location {
	pub file: String,
	pub line: u32,
	pub column: u32,
}

impl Location {
	#[must_use]
	#[track_caller]
	pub fn caller() -> Location {
		std::panic::Location::caller().into()
	}
}

impl std::fmt::Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{}", self.file, self.line, self.column)
	}
}

impl<'a> From<&'a std::panic::Location<'a>> for Location {
	fn from(location: &'a std::panic::Location<'a>) -> Location {
		Location {
			file: location.file().to_owned(),
			line: location.line(),
			column: location.column(),
		}
	}
}

impl Error {
	#[must_use]
	#[track_caller]
	pub fn wrap<C>(self, message: C) -> Error
	where
		C: std::fmt::Display,
	{
		self.wrap_with(|| message)
	}

	#[must_use]
	#[track_caller]
	pub fn wrap_with<C, F>(self, f: F) -> Error
	where
		C: std::fmt::Display,
		F: FnOnce() -> C,
	{
		Error::Message {
			message: f().to_string(),
			location: Location::caller(),
			source: Some(Arc::new(self)),
		}
	}
}

pub trait WrapErr<T, E>: Sized {
	#[track_caller]
	fn wrap_err<C>(self, context: C) -> Result<T, Error>
	where
		C: std::fmt::Display,
	{
		self.wrap_err_with(|| context)
	}

	#[track_caller]
	fn wrap_err_with<C, F>(self, f: F) -> Result<T, Error>
	where
		C: std::fmt::Display,
		F: FnOnce() -> C;
}

impl<T, E> WrapErr<T, E> for Result<T, E>
where
	E: Into<Error>,
{
	#[track_caller]
	fn wrap_err_with<C, F>(self, f: F) -> Result<T, Error>
	where
		C: std::fmt::Display,
		F: FnOnce() -> C,
	{
		match self {
			Ok(value) => Ok(value),
			Err(error) => Err(error.into().wrap_with(f)),
		}
	}
}

impl<T> WrapErr<T, Error> for Option<T> {
	#[track_caller]
	fn wrap_err_with<C, F>(self, f: F) -> Result<T, Error>
	where
		C: std::fmt::Display,
		F: FnOnce() -> C,
	{
		match self {
			Some(value) => Ok(value),
			None => Err(Error::message(f())),
		}
	}
}

#[macro_export]
macro_rules! error {
	($($t:tt)*) => {{
		$crate::error::Error::message(format!($($t)*))
	}};
}

pub use error;

#[allow(clippy::module_name_repetitions)]
#[macro_export]
macro_rules! return_error {
	($($t:tt)*) => {{
		return $crate::error::Result::Err($crate::error!($($t)*))
	}};
}

#[allow(clippy::module_name_repetitions)]
pub use return_error;
