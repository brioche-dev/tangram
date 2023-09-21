use crate::value;
use std::collections::BTreeMap;

crate::id!(Object);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Object);

pub type Value = BTreeMap<String, value::Handle>;

crate::value!(Object);

pub type Data = BTreeMap<String, crate::Id>;
