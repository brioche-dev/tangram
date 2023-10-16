use derive_more::Display;
use std::sync::Arc;
use thiserror::Error;

/// A result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error.
#[derive(Clone, Debug, Display)]
pub struct Error(Arc<Inner>);

/// A message error.
#[derive(
	Clone,
	Debug,
	Error,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[error("{message}")]
struct Inner {
	#[tangram_serialize(id = 0)]
	message: String,
	#[tangram_serialize(id = 1)]
	location: Option<Location>,
	#[tangram_serialize(id = 2)]
	stack: Option<Vec<Location>>,
	#[tangram_serialize(id = 3)]
	source: Option<Error>,
}

/// An error location.
#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Location {
	#[tangram_serialize(id = 0)]
	pub file: String,
	#[tangram_serialize(id = 1)]
	pub line: u32,
	#[tangram_serialize(id = 2)]
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
	#[must_use]
	pub fn new(
		message: String,
		location: Option<Location>,
		stack: Option<Vec<Location>>,
		source: Option<Error>,
	) -> Self {
		Self(Arc::new(Inner {
			message,
			location,
			stack,
			source,
		}))
	}

	#[track_caller]
	pub fn with_message(message: impl std::fmt::Display) -> Self {
		Self(Arc::new(Inner {
			message: message.to_string(),
			location: Some(std::panic::Location::caller().into()),
			stack: None,
			source: None,
		}))
	}

	pub fn with_error(error: impl std::error::Error) -> Self {
		Self(Arc::new(Inner {
			message: error.to_string(),
			location: None,
			stack: None,
			source: error.source().map(Into::into),
		}))
	}

	#[must_use]
	pub fn message(&self) -> &String {
		&self.0.message
	}

	#[must_use]
	pub fn location(&self) -> &Option<Location> {
		&self.0.location
	}

	#[must_use]
	pub fn stack(&self) -> &Option<Vec<Location>> {
		&self.0.stack
	}

	#[must_use]
	pub fn source(&self) -> &Option<Error> {
		&self.0.source
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
		Self(Arc::new(Inner {
			message: f().to_string(),
			location: Some(std::panic::Location::caller().into()),
			stack: None,
			source: Some(self),
		}))
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
		Arc::try_unwrap(error.0)
			.unwrap_or_else(|error| error.as_ref().clone())
			.into()
	}
}

impl<E> From<E> for Error
where
	E: std::error::Error,
{
	fn from(error: E) -> Self {
		Self::with_error(error)
	}
}

impl From<&(dyn std::error::Error + 'static)> for Inner {
	fn from(value: &(dyn std::error::Error + 'static)) -> Self {
		if let Some(value) = value.downcast_ref::<Self>() {
			value.clone()
		} else {
			Self {
				message: value.to_string(),
				location: None,
				stack: None,
				source: value.source().map(Into::into),
			}
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
		let mut error = self.0;
		loop {
			if !first {
				writeln!(f)?;
			}
			first = false;
			let message = error.message();
			write!(f, "{message}")?;
			if let Some(location) = error.location() {
				write!(f, " {location}")?;
			}
			for location in error.stack().iter().flatten() {
				writeln!(f)?;
				write!(f, "  {location}")?;
			}
			if let Some(source) = error.source() {
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
		self.0.serialize(serializer)
	}
}

impl<'de> serde::Deserialize<'de> for Error {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(Self(<_>::deserialize(deserializer)?))
	}
}

impl tangram_serialize::Serialize for Error {
	fn serialize<W>(&self, serializer: &mut tangram_serialize::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		self.0.serialize(serializer)
	}
}

impl tangram_serialize::Deserialize for Error {
	fn deserialize<R>(
		deserializer: &mut tangram_serialize::Deserializer<R>,
	) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		Ok(Self(deserializer.deserialize()?))
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
