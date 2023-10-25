#[derive(
	Debug,
	Clone,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Mutation {
	#[tangram_serialize(id = 0)]
	Unset(()),
	#[tangram_serialize(id = 1)]
	Set(Value),
	#[tangram_serialize(id = 2)]
	SetIfUnset(Value),
	#[tangram_serialize(id = 3)]
	ArrayPrepend(ArrayMutation),
	#[tangram_serialize(id = 4)]
	ArrayAppend(ArrayMutation),
	#[tangram_serialize(id = 5)]
	TemplatePrepend(Thing2),
	#[tangram_serialize(id = 6)]
	TemplateAppend(Thing2),
}

#[derive(
	Debug,
	Clone,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct ArrayMutation {
	#[tangram_serialize(id = 0)]
	value: Template,
}

#[derive(
	Debug,
	Clone,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct TemplateMutation {
	#[tangram_serialize(id = 0)]
	value: Template,
	#[tangram_serialize(id = 1)]
	separator: Template,
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
	Set(Value::Id),
	#[tangram_serialize(id = 2)]
	SetIfUnset(Value::Id),
	#[tangram_serialize(id = 3)]
	ArrayPrepend(ArrayMutation),
	#[tangram_serialize(id = 4)]
	ArrayAppend(ArrayMutation),
	#[tangram_serialize(id = 5)]
	TemplatePrepend(Thing2),
	#[tangram_serialize(id = 6)]
	TemplateAppend(Thing2),
}
