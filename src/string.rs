pub type String = std::string::String;

crate::id!(String);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(String);

pub type Value = std::string::String;

crate::value!(String);

pub type Data = std::string::String;
