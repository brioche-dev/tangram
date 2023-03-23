use std::panic::Location;
use thiserror::Error;

// A result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error.
#[derive(Debug, Error)]
pub enum Error {
	/// An error with a message.
	#[error("{message}\n  {location}")]
	Message {
		message: String,
		location: &'static Location<'static>,
		source: Option<Box<Error>>,
	},

	/// An IO error.
	#[error(transparent)]
	Io(#[from] std::io::Error),

	/// A tangram error.
	#[error(transparent)]
	Tangram(#[from] tangram::error::Error),

	/// Any other error.
	#[error(transparent)]
	Other(Box<dyn std::error::Error + Send + Sync + 'static>),
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

	pub fn other(error: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>) -> Error {
		Error::Other(error.into())
	}
}

impl Error {
	#[allow(dead_code)]
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
			source: Some(Box::new(self)),
		}
	}
}

pub trait WrapErr<T>: Sized {
	#[track_caller]
	fn wrap_err<M>(self, message: M) -> Result<T, Error>
	where
		M: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
	{
		self.wrap_err_with(|| message)
	}

	#[track_caller]
	fn wrap_err_with<M, F>(self, f: F) -> Result<T, Error>
	where
		M: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
		F: FnOnce() -> M;
}

impl<T, E> WrapErr<T> for Result<T, E>
where
	E: Into<Error>,
{
	#[track_caller]
	fn wrap_err_with<C, F>(self, f: F) -> Result<T, Error>
	where
		C: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
		F: FnOnce() -> C,
	{
		match self {
			Ok(value) => Ok(value),
			Err(error) => Err(error.into().wrap_with(f)),
		}
	}
}

impl<T> WrapErr<T> for Option<T> {
	#[track_caller]
	fn wrap_err_with<M, F>(self, f: F) -> Result<T, Error>
	where
		M: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
		F: FnOnce() -> M,
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
