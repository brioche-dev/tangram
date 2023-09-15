use std::collections::BTreeMap;

crate::id!(Object);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(Object);

pub type Value = BTreeMap<String, crate::Handle>;

crate::value!(Object);

pub type Data = BTreeMap<String, crate::Id>;
