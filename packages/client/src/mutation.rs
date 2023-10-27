use crate::{template, value, Template, Value};

#[derive(Debug, Clone)]
pub enum Mutation {
	Unset,
	Set {
		value: Box<Value>,
	},
	SetIfUnset {
		value: Box<Value>,
	},
	ArrayPrepend {
		values: Vec<Value>,
	},
	ArrayAppend {
		values: Vec<Value>,
	},
	TemplatePrepend {
		template: Template,
		separator: Template,
	},
	TemplateAppend {
		template: Template,
		separator: Template,
	},
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum Data {
	Unset(()),
	Set(Box<value::Data>),
	SetIfUnset(Box<value::Data>),
	ArrayPrepend(Vec<value::Data>),
	ArrayAppend(Vec<value::Data>),
	TemplatePrepend(TemplateMutationData),
	TemplateAppend(TemplateMutationData),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TemplateMutationData {
	template: template::Data,
	separator: template::Data,
}

impl Mutation {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Unset => Data::Unset(()),
			Self::Set { value } => Data::Set(Box::new(value::Data::from(value.as_ref().clone()))),
			Self::SetIfUnset { value } => {
				Data::SetIfUnset(Box::new(value::Data::from(value.as_ref().clone())))
			},
			Self::ArrayPrepend { values } => {
				Data::ArrayPrepend(values.iter().cloned().map(Into::into).collect())
			},
			Self::ArrayAppend { values } => {
				Data::ArrayAppend(values.iter().cloned().map(Into::into).collect())
			},
			Self::TemplatePrepend {
				template,
				separator,
			} => Data::TemplatePrepend(TemplateMutationData {
				template: template.to_data(),
				separator: separator.to_data(),
			}),
			Self::TemplateAppend {
				template,
				separator,
			} => Data::TemplateAppend(TemplateMutationData {
				template: template.to_data(),
				separator: separator.to_data(),
			}),
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		match data {
			Data::Unset(()) => Self::Unset,
			Data::Set(data) => Self::Set {
				value: Box::new(Value::from(data.as_ref().clone())),
			},
			Data::SetIfUnset(data) => Self::SetIfUnset {
				value: Box::new(Value::from(data.as_ref().clone())),
			},
			Data::ArrayPrepend(data) => Self::ArrayPrepend {
				values: data.into_iter().map(Into::into).collect(),
			},
			Data::ArrayAppend(data) => Self::ArrayAppend {
				values: data.into_iter().map(Into::into).collect(),
			},
			Data::TemplatePrepend(data) => Self::TemplatePrepend {
				template: Template::from_data(data.template),
				separator: Template::from_data(data.separator),
			},
			Data::TemplateAppend(data) => Self::TemplateAppend {
				template: Template::from_data(data.template),
				separator: Template::from_data(data.separator),
			},
		}
	}
}

impl std::fmt::Display for Mutation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let s = match self {
			Self::Unset => "unset".to_owned(),
			Self::Set { value } => format!("set {value}"),
			Self::SetIfUnset { value } => format!("set-if-unset {value}"),
			Self::ArrayPrepend { values } => format!(
				"array-prepend [{}]",
				values.iter().fold(String::new(), |acc, value| {
					let mut ret = acc.clone();
					if !acc.is_empty() {
						ret.push_str(", ");
					}
					ret.push_str(&value.to_string());
					ret
				})
			),
			Self::ArrayAppend { values } => format!(
				"array-append [{}]",
				values.iter().fold(String::new(), |acc, value| {
					let mut ret = acc.clone();
					if !acc.is_empty() {
						ret.push_str(", ");
					}
					ret.push_str(&value.to_string());
					ret
				})
			),
			Self::TemplatePrepend {
				template,
				separator,
			} => {
				format!("template-prepend {template} ({separator})",)
			},
			Self::TemplateAppend {
				template,
				separator,
			} => {
				format!("template-append {template} ({separator})",)
			},
		};
		write!(f, "(tg.mutation {s})")
	}
}
