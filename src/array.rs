use crate::value;

crate::id!(Array);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Array);

pub type Value = Vec<value::Handle>;

crate::value!(Array);

pub type Data = Vec<crate::Id>;

pub type Array = Vec<value::Handle>;
