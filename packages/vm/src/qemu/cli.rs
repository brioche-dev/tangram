//! Build `qemu` command line arguments from structured values.

use crate::qemu;
use std::fmt;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

/// Short-form for defining an `Arg`
#[macro_export]
macro_rules! arg {
	(-$name:ident) => { Arg::of(stringify!($name)) };
	(-$name:ident, $first:expr) => {{
		let mut a = $crate::qemu::cli::Arg::of(stringify!($name));
		a.first($first);
		a
	}};
	(-$name:ident, $($k:literal = $v:expr),+ $(,)?) => {{
		let mut a = $crate::qemu::cli::Arg::of(stringify!($name));
		$( a.param($k, $v); )*
		a
	}};
	(-$name:ident, $first:expr, $($k:literal = $v:expr),+ $(,)?) => {{
		let mut a = $crate::qemu::cli::Arg::of(stringify!($name));
		a.first($first);
		$( a.param($k, $v); )*
		a
	}};
}

/// Represents a qemu argument, like `-machine 4,sockets=1,cores=4`
#[derive(Clone, PartialEq, Eq)]
pub struct Arg {
	/// The argument name, like `machine`
	name: String,

	/// The optional first argument value, like `4`
	first: Option<String>,

	/// The optional argument parameters, like `sockets=1,cores=4`
	rest: BTreeMap<String, String>,
}

#[allow(clippy::tabs_in_doc_comments)]
/// Represents a qemu argument.
///
/// Note: the parameters are sorted by key.
///
/// ```
/// use tangram_vm::{arg,qemu::cli::Arg};
///
/// // Represent the qemu argument `-smp 4,sockets=1,cores=4,threads=1`
/// let mut arg1 = Arg::of("smp");
/// arg1
///	    .first(4)
///	    .param("cores", 4)
///	    .param("sockets", 1)
///	    .param("threads", 1);
///
/// // You can also use the `arg!` shorthand macro
/// let arg2 = arg!(-smp, 4, "cores" = 4, "sockets" = 1, "threads" = 1);
///
/// assert_eq!(arg1, arg2);
/// assert_eq!(format!("{:?}", arg1), r#"-smp "4,cores=4,sockets=1,threads=1,""#);
/// ```
impl Arg {
	/// Create a flag argument, like `-daemonize`
	#[must_use]
	pub fn of(name: &str) -> Arg {
		Arg {
			name: name.to_string(),
			first: None,
			rest: BTreeMap::new(),
		}
	}

	/// Adds a value (the first term of the comma-separated arg string)
	pub fn first(&mut self, val: impl ArgString) -> &mut Self {
		self.first = Some(val.to_arg_string());
		self
	}

	/// Adds a parameter, like `sockets=1`
	pub fn param(&mut self, key: &str, val: impl ArgString) -> &mut Self {
		self.rest.insert(key.to_string(), val.to_arg_string());
		self
	}

	/// Format the argument as qemu argv entries.
	#[must_use]
	pub fn as_argv(&self) -> [String; 2] {
		let param_name = format!("-{}", arg_escape(&self.name));
		let mut value = String::new();

		if let Some(first) = &self.first {
			value.push_str(first.as_str());
		}
		if self.first.is_some() && !self.rest.is_empty() {
			value.push(',');
		}

		for (k, v) in &self.rest {
			use std::fmt::Write;
			let _ = write!(&mut value, "{}={}", arg_escape(k), arg_escape(v));
			value.push(',');
		}

		[param_name, value]
	}
}

impl fmt::Debug for Arg {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let [param_name, value] = self.as_argv();
		write!(f, "{} {:?}", param_name, value)
	}
}

/// Escape commas in a qemu argument by doubling them.
fn arg_escape(source: &str) -> String {
	source.replace(',', ",,")
}

impl ArgString for qemu::ImageFormat {
	fn to_arg_string(&self) -> String {
		use qemu::ImageFormat::{Qcow2, Raw};
		match self {
			Raw => "raw".into(),
			Qcow2 => "qcow2".into(),
		}
	}
}

/// Any value that can be converted to a qemu parameter value. For instance, `bool`s become `on|off`.
pub trait ArgString {
	fn to_arg_string(&self) -> String;
}
impl ArgString for bool {
	fn to_arg_string(&self) -> String {
		match self {
			true => "on",
			false => "off",
		}
		.to_string()
	}
}

impl ArgString for &Path {
	fn to_arg_string(&self) -> String {
		// XXX: propagate this error without panicking
		self.to_str()
			.expect("non-Unicode paths not supported")
			.to_owned()
	}
}
impl ArgString for PathBuf {
	fn to_arg_string(&self) -> String {
		// XXX: propagate this error without panicking
		self.to_str()
			.expect("non-Unicode paths not supported")
			.to_owned()
	}
}

impl<AS: ArgString> ArgString for &AS {
	fn to_arg_string(&self) -> String {
		(*self).to_arg_string()
	}
}

macro_rules! _impl_arg_string {
	($t:ty) => {
		impl ArgString for $t {
			fn to_arg_string(&self) -> String {
				self.to_string()
			}
		}
	};
}
_impl_arg_string!(String);
_impl_arg_string!(&str);
_impl_arg_string!(u8);
_impl_arg_string!(u16);
_impl_arg_string!(u32);
_impl_arg_string!(u64);
_impl_arg_string!(i8);
_impl_arg_string!(i16);
_impl_arg_string!(i32);
_impl_arg_string!(i64);
_impl_arg_string!(usize);
_impl_arg_string!(isize);
