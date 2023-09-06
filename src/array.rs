use crate::any;

crate::id!();

crate::kind!(Array);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

pub type Value = Vec<any::Handle>;

pub type Data = Vec<any::Id>;

pub type Array = Vec<any::Handle>;
