use crate::{template, value, Template, Value};
use std::fmt;

#[derive(Debug, Clone)]
pub enum Mutation {
	Unset(()),
	Set(Box<Value>),
	SetIfUnset(Box<Value>),
	ArrayPrepend(ArrayMutation),
	ArrayAppend(ArrayMutation),
	TemplatePrepend(TemplateMutation),
	TemplateAppend(TemplateMutation),
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
			Self::ArrayPrepend(mutation) => Data::ArrayPrepend(mutation.to_data()),
			Self::ArrayAppend(mutation) => Data::ArrayAppend(mutation.to_data()),
			Self::TemplatePrepend(mutation) => Data::TemplatePrepend(mutation.to_data()),
			Self::TemplateAppend(mutation) => Data::TemplateAppend(mutation.to_data()),
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
			Data::ArrayPrepend(data) => Self::ArrayPrepend(ArrayMutation::from_data(data)),
			Data::ArrayAppend(data) => Self::ArrayAppend(ArrayMutation::from_data(data)),
			Data::TemplatePrepend(data) => Self::TemplatePrepend(TemplateMutation::from_data(data)),
			Data::TemplateAppend(data) => Self::TemplateAppend(TemplateMutation::from_data(data)),
		}
	}
}

impl fmt::Display for Mutation {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let s = match self {
			Self::Unset(()) => "unset".to_owned(),
			Self::Set(value) => format!("set {value}"),
			Self::SetIfUnset(value) => format!("set-if-unset {value}"),
			Self::ArrayPrepend(mutation) => format!("array-prepend {}", mutation.value),
			Self::ArrayAppend(mutation) => format!("array-append {}", mutation.value),
			Self::TemplatePrepend(mutation) => {
				format!(
					"template-prepend {} ({})",
					mutation.value, mutation.separator
				)
			},
			Self::TemplateAppend(mutation) => {
				format!(
					"template-append {} ({})",
					mutation.value, mutation.separator
				)
			},
		};
		write!(f, "(tg.mutation {s})")
	}
}

#[derive(Debug, Clone)]
pub struct ArrayMutation {
	pub value: Template,
}

impl ArrayMutation {
	#[must_use]
	pub fn to_data(&self) -> ArrayMutationData {
		ArrayMutationData {
			value: self.value.to_data(),
		}
	}

	#[must_use]
	pub fn from_data(data: ArrayMutationData) -> Self {
		Self {
			value: Template::from_data(data.value),
		}
	}
}

#[derive(Debug, Clone)]
pub struct TemplateMutation {
	pub value: Template,
	pub separator: Template,
}

impl TemplateMutation {
	#[must_use]
	pub fn to_data(&self) -> TemplateMutationData {
		TemplateMutationData {
			value: self.value.to_data(),
			separator: self.separator.to_data(),
		}
	}

	#[must_use]
	pub fn from_data(data: TemplateMutationData) -> Self {
		Self {
			value: Template::from_data(data.value),
			separator: Template::from_data(data.separator),
		}
	}
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
	ArrayPrepend(ArrayMutationData),
	#[tangram_serialize(id = 4)]
	ArrayAppend(ArrayMutationData),
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
pub struct ArrayMutationData {
	#[tangram_serialize(id = 0)]
	value: template::Data,
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
