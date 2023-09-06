pub type String = std::string::String;

crate::id!();

crate::kind!(String);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

pub type Value = std::string::String;

pub type Data = std::string::String;
