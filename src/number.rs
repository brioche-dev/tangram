use crate::value;

pub type Number = f64;

crate::id!(Number);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Number);

pub type Value = f64;

crate::value!(Number);

pub type Data = f64;
