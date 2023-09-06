pub type Bool = bool;

crate::id!();

crate::kind!(Bool);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

pub type Value = bool;

pub type Data = bool;
