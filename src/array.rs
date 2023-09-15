crate::id!(Array);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(Array);

pub type Value = Vec<crate::Handle>;

crate::value!(Array);

pub type Data = Vec<crate::Id>;

pub type Array = Vec<crate::Handle>;
