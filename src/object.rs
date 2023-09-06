use crate::any;
use std::collections::BTreeMap;

crate::id!();

crate::kind!(Object);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

pub type Value = BTreeMap<String, any::Handle>;

pub type Data = BTreeMap<String, any::Id>;
