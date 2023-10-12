use derive_more::Display;
use std::sync::Arc;
use thiserror::Error;

/// A result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error.
#[derive(Debug, Display)]
pub struct Error(Box<dyn std::error::Error + Send + Sync + 'static>);

/// A message error.
#[derive(Clone, Debug, Error, serde::Deserialize, serde::Serialize)]
#[error("{message}")]
pub struct Message {
	pub message: String,
	pub location: Option<Location>,
	pub stack: Option<Vec<Location>>,
	pub source: Option<Arc<Error>>,
}

/// An error location.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Location {
	pub file: String,
	pub line: u32,
	pub column: u32,
}

/// An error trace.
pub struct Trace<'a>(&'a Error);

/// An extension trait for wrapping an error with a message.
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
	pub fn with_message(message: impl std::fmt::Display) -> Self {
		Message {
			message: message.to_string(),
			location: Some(std::panic::Location::caller().into()),
			stack: None,
			source: None,
		}
		.into()
	}

	#[must_use]
	pub fn trace(&self) -> Trace {
		Trace(self)
	}

	#[must_use]
	#[track_caller]
	pub fn wrap<C>(self, message: C) -> Self
	where
		C: std::fmt::Display,
	{
		self.wrap_with(|| message)
	}

	#[must_use]
	#[track_caller]
	pub fn wrap_with<C, F>(self, f: F) -> Self
	where
		C: std::fmt::Display,
		F: FnOnce() -> C,
	{
		Message {
			message: f().to_string(),
			location: Some(std::panic::Location::caller().into()),
			stack: None,
			source: Some(Arc::new(self)),
		}
		.into()
	}
}

impl std::ops::Deref for Error {
	type Target = dyn std::error::Error + Send + Sync + 'static;

	fn deref(&self) -> &Self::Target {
		&*self.0
	}
}

impl From<Error> for Box<dyn std::error::Error + Send + Sync + 'static> {
	fn from(error: Error) -> Self {
		error.0
	}
}

impl<E> From<E> for Error
where
	E: std::error::Error + Send + Sync + 'static,
{
	fn from(error: E) -> Self {
		Self(Box::new(error))
	}
}

impl From<Error> for Message {
	fn from(value: Error) -> Self {
		match value.0.downcast() {
			Ok(message) => *message,
			Err(error) => error.as_ref().into(),
		}
	}
}

impl From<&(dyn std::error::Error + Send + Sync + 'static)> for Message {
	fn from(value: &(dyn std::error::Error + Send + Sync + 'static)) -> Self {
		Self {
			message: value.to_string(),
			location: None,
			stack: None,
			source: value
				.source()
				.map(Message::from)
				.map(Error::from)
				.map(Arc::new),
		}
	}
}

impl From<&(dyn std::error::Error + 'static)> for Message {
	fn from(value: &(dyn std::error::Error + 'static)) -> Self {
		Self {
			message: value.to_string(),
			location: None,
			stack: None,
			source: value
				.source()
				.map(Message::from)
				.map(Error::from)
				.map(Arc::new),
		}
	}
}

impl<'a> From<&'a std::panic::Location<'a>> for Location {
	fn from(location: &'a std::panic::Location<'a>) -> Self {
		Self {
			file: location.file().to_owned(),
			line: location.line() - 1,
			column: location.column() - 1,
		}
	}
}

impl<'a> std::fmt::Display for Trace<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut first = true;
		let mut error = &*(self.0).0 as &(dyn std::error::Error + 'static);
		loop {
			if !first {
				writeln!(f)?;
			}
			first = false;
			if let Some(error) = error.downcast_ref::<Message>() {
				let message = &error.message;
				write!(f, "{message}")?;
				if let Some(location) = &error.location {
					write!(f, " {location}")?;
				}
				for location in error.stack.iter().flatten() {
					writeln!(f)?;
					write!(f, "  {location}")?;
				}
			} else {
				write!(f, "{error}")?;
			}
			if let Some(source) = std::error::Error::source(error) {
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
		write!(f, "{}:{}:{}", self.file, self.line + 1, self.column + 1)
	}
}

impl serde::Serialize for Error {
	fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		Message::from(self.0.as_ref()).serialize(serializer)
	}
}

impl<'de> serde::Deserialize<'de> for Error {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Message::deserialize(deserializer).map(Into::into)
	}
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
			None => Err(Error::with_message(f())),
		}
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
