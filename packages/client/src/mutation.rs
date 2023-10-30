use crate::{object, template, value, Template, Value};

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
	Unset,
	Set {
		value: Box<value::Data>,
	},
	SetIfUnset {
		value: Box<value::Data>,
	},
	ArrayPrepend {
		values: Vec<value::Data>,
	},
	ArrayAppend {
		values: Vec<value::Data>,
	},
	TemplatePrepend {
		template: template::Data,
		separator: template::Data,
	},
	TemplateAppend {
		template: template::Data,
		separator: template::Data,
	},
}

impl Mutation {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Unset => Data::Unset,
			Self::Set { value } => Data::Set {
				value: Box::new(value.to_data()),
			},
			Self::SetIfUnset { value } => Data::SetIfUnset {
				value: Box::new(value.to_data()),
			},
			Self::ArrayPrepend { values } => Data::ArrayPrepend {
				values: values.iter().map(Value::to_data).collect(),
			},
			Self::ArrayAppend { values } => Data::ArrayAppend {
				values: values.iter().map(Value::to_data).collect(),
			},
			Self::TemplatePrepend {
				template,
				separator,
			} => Data::TemplatePrepend {
				template: template.to_data(),
				separator: separator.to_data(),
			},
			Self::TemplateAppend {
				template,
				separator,
			} => Data::TemplateAppend {
				template: template.to_data(),
				separator: separator.to_data(),
			},
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		match data {
			Data::Unset => Self::Unset,
			Data::Set { value } => Self::Set {
				value: Box::new(Value::from(value.as_ref().clone())),
			},
			Data::SetIfUnset { value } => Self::SetIfUnset {
				value: Box::new(Value::from(value.as_ref().clone())),
			},
			Data::ArrayPrepend { values } => Self::ArrayPrepend {
				values: values.into_iter().map(Into::into).collect(),
			},
			Data::ArrayAppend { values } => Self::ArrayAppend {
				values: values.into_iter().map(Into::into).collect(),
			},
			Data::TemplatePrepend {
				template,
				separator,
			} => Self::TemplatePrepend {
				template: Template::from_data(template),
				separator: Template::from_data(separator),
			},
			Data::TemplateAppend {
				template,
				separator,
			} => Self::TemplateAppend {
				template: Template::from_data(template),
				separator: Template::from_data(separator),
			},
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		match self {
			Self::Unset => vec![],
			Self::Set { value } | Self::SetIfUnset { value } => value.children(),
			Self::ArrayPrepend { values } | Self::ArrayAppend { values } => {
				values.iter().flat_map(Value::children).collect()
			},
			Self::TemplatePrepend {
				template,
				separator,
			}
			| Self::TemplateAppend {
				template,
				separator,
			} => template
				.children()
				.into_iter()
				.chain(separator.children())
				.collect(),
		}
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Self::Unset => vec![],
			Self::Set { value } | Self::SetIfUnset { value } => value.children(),
			Self::ArrayPrepend { values } | Self::ArrayAppend { values } => {
				values.iter().flat_map(value::Data::children).collect()
			},
			Self::TemplatePrepend {
				template,
				separator,
			}
			| Self::TemplateAppend {
				template,
				separator,
			} => template
				.children()
				.into_iter()
				.chain(separator.children())
				.collect(),
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
