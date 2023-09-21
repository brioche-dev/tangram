use crate::value;

pub type Bool = bool;

crate::id!(Bool);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Bool);

pub type Value = bool;

crate::value!(Bool);

pub type Data = bool;
