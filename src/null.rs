use crate::value;

crate::id!(Null);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Null);

pub type Value = ();

crate::value!(Null);

pub type Data = ();
