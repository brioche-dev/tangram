use super::Struct;
use quote::quote;

impl<'a> Struct<'a> {
	pub fn deserialize(self) -> proc_macro2::TokenStream {
		// Get the ident.
		let ident = self.ident;

		// Generate the body.
		let body = if let Some(try_from) = self.try_from {
			quote! {
				let value = #try_from::deserialize(deserializer)?;
				let value = value.try_into().map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
				Ok(value)
			}
		} else {
			// Get the field ids.
			let field_ids = self.fields.iter().map(|field| field.id).collect::<Vec<_>>();

			// Get the field idents.
			let field_idents = self
				.fields
				.iter()
				.map(|field| &field.ident)
				.collect::<Vec<_>>();

			quote! {
				// Read the kind.
				deserializer.ensure_kind(buffalo::Kind::Struct)?;

				// Initialize the fields.
				#(let mut #field_idents = None;)*

				// Read the number of serialized fields.
				let len = deserializer.read_uvarint()?;

				// Deserialize `len` fields.
				for _ in 0..len {
					// Deserialize the field id.
					let field_id = deserializer.read_id()?;

					// Deserialize the field value.
					match field_id {
						#(#field_ids => { #field_idents = Some(deserializer.deserialize()?); })*

						// Skip over fields with unknown ids.
						_ => {
							buffalo::Value::deserialize(deserializer)?;
						},
					}
				}

				// Retrieve the fields.
				#(let #field_idents = #field_idents.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Missing field."))?;)*

				// Create the struct.
				Ok(#ident {
					#(#field_idents,)*
				})
			}
		};

		// Generate the code.
		let code = quote! {
			impl buffalo::Deserialize for #ident {
				fn deserialize<R>(deserializer: &mut buffalo::Deserializer<R>) -> std::io::Result<Self>
				where
					R: std::io::Read,
				{
					#body
				}
			}
		};

		code
	}
}
