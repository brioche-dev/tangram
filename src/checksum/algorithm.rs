use crate::{
	error::{return_error, Error, Result},
	target::{FromV8, ToV8},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Algorithm {
	Sha256,
	Sha512,
	Blake3,
}

impl std::fmt::Display for Algorithm {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let system = match self {
			Algorithm::Sha256 => "sha256",
			Algorithm::Sha512 => "sha512",
			Algorithm::Blake3 => "blake3",
		};
		write!(f, "{system}")?;
		Ok(())
	}
}

impl std::str::FromStr for Algorithm {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let system = match s {
			"sha256" => Algorithm::Sha256,
			"sha512" => Algorithm::Sha512,
			"blake3" => Algorithm::Blake3,
			_ => return_error!(r#"Invalid algorithm "{s}"."#),
		};
		Ok(system)
	}
}

impl From<Algorithm> for String {
	fn from(value: Algorithm) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Algorithm {
	type Error = Error;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		value.parse()
	}
}

impl ToV8 for Algorithm {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		serde_v8::to_v8(scope, self).map_err(Error::other)
	}
}

impl FromV8 for Algorithm {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		serde_v8::from_v8(scope, value).map_err(Error::other)
	}
}
