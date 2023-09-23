use crate::value;

crate::id!(Array);

crate::handle!(Array);

pub type Value = Vec<value::Handle>;

pub type Data = Vec<crate::Id>;
