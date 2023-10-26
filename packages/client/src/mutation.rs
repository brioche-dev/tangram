use crate::{template, value, Template, Value};

#[derive(Debug, Clone)]
pub enum Mutation {
	Unset(()),
	Set(Box<Value>),
	SetIfUnset(Box<Value>),
	ArrayPrepend(Vec<Value>),
	ArrayAppend(Vec<Value>),
	TemplatePrepend {
		value: Template,
		separator: Template,
	},
	TemplateAppend {
		value: Template,
		separator: Template,
	},
}

#[derive(
	Debug,
	Clone,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Unset(()),
	#[tangram_serialize(id = 1)]
	Set(Box<value::Data>),
	#[tangram_serialize(id = 2)]
	SetIfUnset(Box<value::Data>),
	#[tangram_serialize(id = 3)]
	ArrayPrepend(Vec<value::Data>),
	#[tangram_serialize(id = 4)]
	ArrayAppend(Vec<value::Data>),
	#[tangram_serialize(id = 5)]
	TemplatePrepend(TemplateMutationData),
	#[tangram_serialize(id = 6)]
	TemplateAppend(TemplateMutationData),
}

#[derive(
	Debug,
	Clone,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct TemplateMutationData {
	#[tangram_serialize(id = 0)]
	value: template::Data,
	#[tangram_serialize(id = 1)]
	separator: template::Data,
}

impl Mutation {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Unset(()) => Data::Unset(()),
			Self::Set(value) => Data::Set(Box::new(value::Data::from(value.as_ref().clone()))),
			Self::SetIfUnset(value) => {
				Data::SetIfUnset(Box::new(value::Data::from(value.as_ref().clone())))
			},
			Self::ArrayPrepend(vec) => {
				Data::ArrayPrepend(vec.iter().cloned().map(Into::into).collect())
			},
			Self::ArrayAppend(vec) => {
				Data::ArrayAppend(vec.iter().cloned().map(Into::into).collect())
			},
			Self::TemplatePrepend { value, separator } => {
				Data::TemplatePrepend(TemplateMutationData {
					value: value.to_data(),
					separator: separator.to_data(),
				})
			},
			Self::TemplateAppend { value, separator } => {
				Data::TemplateAppend(TemplateMutationData {
					value: value.to_data(),
					separator: separator.to_data(),
				})
			},
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		match data {
			Data::Unset(()) => Self::Unset(()),
			Data::Set(data) => Self::Set(Box::new(Value::from(data.as_ref().clone()))),
			Data::SetIfUnset(data) => {
				Self::SetIfUnset(Box::new(Value::from(data.as_ref().clone())))
			},
			Data::ArrayPrepend(data) => {
				Self::ArrayPrepend(data.into_iter().map(Into::into).collect())
			},
			Data::ArrayAppend(data) => {
				Self::ArrayAppend(data.into_iter().map(Into::into).collect())
			},
			Data::TemplatePrepend(data) => Self::TemplatePrepend {
				value: Template::from_data(data.value),
				separator: Template::from_data(data.separator),
			},
			Data::TemplateAppend(data) => Self::TemplateAppend {
				value: Template::from_data(data.value),
				separator: Template::from_data(data.separator),
			},
		}
	}
}

impl std::fmt::Display for Mutation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let s = match self {
			Self::Unset(()) => "unset".to_owned(),
			Self::Set(value) => format!("set {value}"),
			Self::SetIfUnset(value) => format!("set-if-unset {value}"),
			Self::ArrayPrepend(vec) => format!(
				"array-prepend [{}]",
				vec.iter().fold(String::new(), |acc, value| {
					let mut ret = acc.clone();
					if !acc.is_empty() {
						ret.push_str(", ");
					}
					ret.push_str(&value.to_string());
					ret
				})
			),
			Self::ArrayAppend(vec) => format!(
				"array-append [{}]",
				vec.iter().fold(String::new(), |acc, value| {
					let mut ret = acc.clone();
					if !acc.is_empty() {
						ret.push_str(", ");
					}
					ret.push_str(&value.to_string());
					ret
				})
			),
			Self::TemplatePrepend { value, separator } => {
				format!("template-prepend {value} ({separator})",)
			},
			Self::TemplateAppend { value, separator } => {
				format!("template-append {value} ({separator})",)
			},
		};
		write!(f, "(tg.mutation {s})")
	}
}
