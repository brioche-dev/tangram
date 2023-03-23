mod deserialize;
mod parse;
mod serialize;

pub struct Struct<'a> {
	pub visibility: &'a syn::Visibility,
	pub ident: &'a syn::Ident,
	pub into: Option<syn::Type>,
	pub try_from: Option<syn::Type>,
	pub fields: Vec<Field<'a>>,
}

pub struct Field<'a> {
	pub id: Option<u8>,
	pub ident: Option<&'a syn::Ident>,
}
