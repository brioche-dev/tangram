use crate::{object, template, value, Client, Error, Result, Template, Value};
use futures::{stream::FuturesOrdered, TryStreamExt};
use itertools::Itertools;

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
	pub async fn data(&self, client: &dyn Client) -> Result<Data> {
		Ok(match self {
			Self::Unset => Data::Unset,
			Self::Set { value } => Data::Set {
				value: Box::new(value.data(client).await?),
			},
			Self::SetIfUnset { value } => Data::SetIfUnset {
				value: Box::new(value.data(client).await?),
			},
			Self::ArrayPrepend { values } => Data::ArrayPrepend {
				values: values
					.iter()
					.map(|value| value.data(client))
					.collect::<FuturesOrdered<_>>()
					.try_collect()
					.await?,
			},
			Self::ArrayAppend { values } => Data::ArrayAppend {
				values: values
					.iter()
					.map(|value| value.data(client))
					.collect::<FuturesOrdered<_>>()
					.try_collect()
					.await?,
			},
			Self::TemplatePrepend {
				template,
				separator,
			} => Data::TemplatePrepend {
				template: template.data(client).await?,
				separator: separator.data(client).await?,
			},
			Self::TemplateAppend {
				template,
				separator,
			} => Data::TemplateAppend {
				template: template.data(client).await?,
				separator: separator.data(client).await?,
			},
		})
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

impl TryFrom<Data> for Mutation {
	type Error = Error;

	fn try_from(data: Data) -> std::result::Result<Self, Self::Error> {
		Ok(match data {
			Data::Unset => Self::Unset,
			Data::Set { value } => Self::Set {
				value: Box::new((*value).try_into()?),
			},
			Data::SetIfUnset { value } => Self::SetIfUnset {
				value: Box::new((*value).try_into()?),
			},
			Data::ArrayPrepend { values } => Self::ArrayPrepend {
				values: values.into_iter().map(TryInto::try_into).try_collect()?,
			},
			Data::ArrayAppend { values } => Self::ArrayAppend {
				values: values.into_iter().map(TryInto::try_into).try_collect()?,
			},
			Data::TemplatePrepend {
				template,
				separator,
			} => Self::TemplatePrepend {
				template: template.try_into()?,
				separator: separator.try_into()?,
			},
			Data::TemplateAppend {
				template,
				separator,
			} => Self::TemplateAppend {
				template: template.try_into()?,
				separator: separator.try_into()?,
			},
		})
	}
}

impl std::fmt::Display for Mutation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "(tg.mutation)")?;
		Ok(())
	}
}
