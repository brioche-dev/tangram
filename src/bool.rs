pub type Bool = bool;

crate::id!(Bool);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(Bool);

pub type Value = bool;

crate::value!(Bool);

pub type Data = bool;
