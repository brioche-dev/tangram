use std::sync::Arc;
use thiserror::Error;

/// A result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error.
#[derive(Clone, Debug, Error)]
#[error("{message}")]
pub struct Error {
	message: String,
	location: Option<Location>,
	source: Option<Arc<Error>>,
}

#[derive(Clone, Debug)]
pub struct Location {
	pub file: String,
	pub line: u32,
	pub column: u32,
}

pub struct Trace<'a>(&'a Error);

pub trait WrapErr<T, E>: Sized {
	#[track_caller]
	fn wrap_err<M>(self, message: M) -> Result<T, Error>
	where
		M: std::fmt::Display,
	{
		self.wrap_err_with(|| message)
	}

	#[track_caller]
	fn wrap_err_with<C, F>(self, f: F) -> Result<T, Error>
	where
		C: std::fmt::Display,
		F: FnOnce() -> C;
}

impl Error {
	#[track_caller]
	pub fn with_message(message: impl std::fmt::Display) -> Error {
		Error {
			message: message.to_string(),
			location: Some(Location::caller()),
			source: None,
		}
	}

	pub fn with_error(error: impl std::error::Error) -> Self {
		Self {
			message: error.to_string(),
			location: None,
			source: error
				.source()
				.map(|error| Arc::new(Self::with_error(error))),
		}
	}

	#[must_use]
	pub fn trace(&self) -> Trace {
		Trace(self)
	}

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
		Error {
			message: f().to_string(),
			location: Some(Location::caller()),
			source: Some(Arc::new(self)),
		}
	}
}

impl Location {
	#[must_use]
	#[track_caller]
	pub fn caller() -> Location {
		std::panic::Location::caller().into()
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

impl<'a> std::fmt::Display for Trace<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut error = self.0;
		loop {
			writeln!(f, "{error}")?;
			if let Some(location) = &error.location {
				writeln!(f, "  {location}")?;
			}
			if let Some(source) = &error.source {
				error = source;
			} else {
				break;
			}
		}
		Ok(())
	}
}

impl std::fmt::Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{}", self.file, self.line, self.column)
	}
}

impl<T, E> WrapErr<T, E> for Result<T, E>
where
	E: std::error::Error,
{
	#[track_caller]
	fn wrap_err_with<C, F>(self, f: F) -> Result<T, Error>
	where
		C: std::fmt::Display,
		F: FnOnce() -> C,
	{
		match self {
			Ok(value) => Ok(value),
			Err(error) => Err(Error::with_error(error).wrap_with(f)),
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
			None => Err(Error::with_message(f())),
		}
	}
}

impl From<reqwest::Error> for Error {
	fn from(error: reqwest::Error) -> Self {
		Self::with_error(error)
	}
}

#[cfg(feature = "server")]
impl From<lmdb::Error> for Error {
	fn from(error: lmdb::Error) -> Self {
		Self::with_error(error)
	}
}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Error {
		Self::with_error(error)
	}
}

#[macro_export]
macro_rules! error {
	($($t:tt)*) => {{
		$crate::error::Error::with_message(format!($($t)*))
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
