use super::convert::{from_v8, FromV8, ToV8};
use crate::{
	blob, directory, file, package, return_error, symlink, task, template, Artifact, Blob, Bytes,
	Checksum, Directory, Error, File, Package, Placeholder, Relpath, Result, Subpath, Symlink,
	System, Task, Template, Value, WrapErr,
};

impl ToV8 for Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Value::Null(value) => value.to_v8(scope),
			Value::Bool(value) => value.to_v8(scope),
			Value::Number(value) => value.to_v8(scope),
			Value::String(value) => value.to_v8(scope),
			Value::Bytes(value) => value.to_v8(scope),
			Value::Blob(value) => value.to_v8(scope),
			Value::Directory(value) => value.to_v8(scope),
			Value::File(value) => value.to_v8(scope),
			Value::Symlink(value) => value.to_v8(scope),
			Value::Placeholder(value) => value.to_v8(scope),
			Value::Template(value) => value.to_v8(scope),
			Value::Package(value) => value.to_v8(scope),
			Value::Task(value) => value.to_v8(scope),
			Value::Array(value) => value.to_v8(scope),
			Value::Map(value) => value.to_v8(scope),
		}
	}
}

impl FromV8 for Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let blob = v8::String::new_external_onebyte_static(scope, "Blob".as_bytes()).unwrap();
		let blob = tg.get(scope, blob.into()).unwrap();
		let blob = v8::Local::<v8::Function>::try_from(blob).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let placeholder =
			v8::String::new_external_onebyte_static(scope, "Placeholder".as_bytes()).unwrap();
		let placeholder = tg.get(scope, placeholder.into()).unwrap();
		let placeholder = v8::Local::<v8::Function>::try_from(placeholder).unwrap();

		let template =
			v8::String::new_external_onebyte_static(scope, "Template".as_bytes()).unwrap();
		let template = tg.get(scope, template.into()).unwrap();
		let template = v8::Local::<v8::Function>::try_from(template).unwrap();

		let package = v8::String::new_external_onebyte_static(scope, "Package".as_bytes()).unwrap();
		let package = tg.get(scope, package.into()).unwrap();
		let package = v8::Local::<v8::Function>::try_from(package).unwrap();

		let task = v8::String::new_external_onebyte_static(scope, "Task".as_bytes()).unwrap();
		let task = tg.get(scope, task.into()).unwrap();
		let task = v8::Local::<v8::Function>::try_from(task).unwrap();

		if value.is_null_or_undefined() {
			Ok(Value::Null(()))
		} else if value.is_boolean() {
			Ok(Value::Bool(from_v8(scope, value)?))
		} else if value.is_number() {
			Ok(Value::Number(from_v8(scope, value)?))
		} else if value.is_string() {
			Ok(Value::String(from_v8(scope, value)?))
		} else if value.is_uint8_array() {
			Ok(Value::Bytes(from_v8(scope, value)?))
		} else if value.instance_of(scope, blob.into()).unwrap() {
			Ok(Value::Blob(from_v8(scope, value)?))
		} else if value.instance_of(scope, directory.into()).unwrap() {
			Ok(Value::Directory(from_v8(scope, value)?))
		} else if value.instance_of(scope, file.into()).unwrap() {
			Ok(Value::File(from_v8(scope, value)?))
		} else if value.instance_of(scope, symlink.into()).unwrap() {
			Ok(Value::Symlink(from_v8(scope, value)?))
		} else if value.instance_of(scope, placeholder.into()).unwrap() {
			Ok(Value::Placeholder(from_v8(scope, value)?))
		} else if value.instance_of(scope, template.into()).unwrap() {
			Ok(Value::Template(from_v8(scope, value)?))
		} else if value.instance_of(scope, package.into()).unwrap() {
			Ok(Value::Package(from_v8(scope, value)?))
		} else if value.instance_of(scope, task.into()).unwrap() {
			Ok(Value::Task(from_v8(scope, value)?))
		} else if value.is_array() {
			Ok(Value::Array(from_v8(scope, value)?))
		} else if value.is_object() {
			Ok(Value::Map(from_v8(scope, value)?))
		} else {
			return_error!("Invalid value.");
		}
	}
}

impl ToV8 for Bytes {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let value = v8::ArrayBuffer::with_backing_store(scope, self.buffer().backing_store());
		let value =
			v8::Uint8Array::new(scope, value, self.range().start, self.range().len()).unwrap();
		Ok(value.into())
	}
}

impl FromV8 for Bytes {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = v8::Local::<v8::Uint8Array>::try_from(value)
			.map_err(Error::other)
			.wrap_err("Expected a Uint8Array.")?;
		let backing_store = value
			.buffer(scope)
			.wrap_err("Expected the Uint8Array to have a buffer.")?
			.get_backing_store();
		let range = value.byte_offset()..(value.byte_offset() + value.byte_length());
		Ok(Self::new(backing_store.into(), range))
	}
}

impl ToV8 for Relpath {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for Relpath {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_rust_string_lossy(scope);
		let value = value.parse()?;
		Ok(value)
	}
}

impl ToV8 for Subpath {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for Subpath {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_rust_string_lossy(scope);
		let value = value.parse()?;
		Ok(value)
	}
}

impl ToV8 for Blob {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let blob = v8::String::new_external_onebyte_static(scope, "Blob".as_bytes()).unwrap();
		let blob = tg.get(scope, blob.into()).unwrap();
		let blob = v8::Local::<v8::Function>::try_from(blob).unwrap();

		let object = blob.new_instance(scope, &[]).unwrap();

		Ok(object.into())
	}
}

impl FromV8 for Blob {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let blob = v8::String::new_external_onebyte_static(scope, "Blob".as_bytes()).unwrap();
		let blob = tg.get(scope, blob.into()).unwrap();
		let blob = v8::Local::<v8::Function>::try_from(blob).unwrap();

		if !value.instance_of(scope, blob.into()).unwrap() {
			return_error!("Expected a blob.");
		}
		let value = value.to_object(scope).unwrap();

		todo!()
	}
}

impl ToV8 for blob::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
		let value = match self {
			blob::Object::Branch(sizes) => sizes.to_v8(scope)?,
			blob::Object::Leaf(size) => size.to_v8(scope)?,
		};
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for blob::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
		// let kind = v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
		// let kind = value.get(scope, kind.into()).unwrap();
		// let kind = if kind.is_array() {
		// 	blob::Object::Branch(from_v8(scope, kind)?)
		// } else if kind.is_number() {
		// 	blob::Object::Leaf(from_v8(scope, kind)?)
		// } else {
		// 	return_error!("Expected a kind.");
		// };

		// Ok(kind)
	}
}

impl ToV8 for Artifact {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Directory(directory) => directory.to_v8(scope),
			Self::File(file) => file.to_v8(scope),
			Self::Symlink(symlink) => symlink.to_v8(scope),
		}
	}
}

impl FromV8 for Artifact {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let artifact = if value.instance_of(scope, directory.into()).unwrap() {
			Self::Directory(from_v8(scope, value)?)
		} else if value.instance_of(scope, file.into()).unwrap() {
			Self::File(from_v8(scope, value)?)
		} else if value.instance_of(scope, symlink.into()).unwrap() {
			Self::Symlink(from_v8(scope, value)?)
		} else {
			return_error!("Expected a directory, file, or symlink.")
		};

		Ok(artifact)
	}
}

impl ToV8 for Directory {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for Directory {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}

impl ToV8 for directory::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let object = directory.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "entries".as_bytes()).unwrap();
		let value = self.entries.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for directory::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		if !value.instance_of(scope, directory.into()).unwrap() {
			return_error!("Expected a directory.");
		}
		let value = value.to_object(scope).unwrap();

		let entries = v8::String::new_external_onebyte_static(scope, "entries".as_bytes()).unwrap();
		let entries = value.get(scope, entries.into()).unwrap();
		let entries = from_v8(scope, entries)?;

		Ok(Self { entries })
	}
}

impl ToV8 for File {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for File {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}

impl ToV8 for file::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let object = file.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "contents".as_bytes()).unwrap();
		let value = self.contents.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let value = self.executable.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "references".as_bytes()).unwrap();
		let value = self.references.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for file::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		if !value.instance_of(scope, file.into()).unwrap() {
			return_error!("Expected a file.");
		}
		let value = value.to_object(scope).unwrap();

		let contents =
			v8::String::new_external_onebyte_static(scope, "contents".as_bytes()).unwrap();
		let contents = value.get(scope, contents.into()).unwrap();
		let contents = from_v8(scope, contents)?;

		let executable =
			v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let executable = value.get(scope, executable.into()).unwrap();
		let executable = from_v8(scope, executable)?;

		let references =
			v8::String::new_external_onebyte_static(scope, "references".as_bytes()).unwrap();
		let references = value.get(scope, references.into()).unwrap();
		let references = from_v8(scope, references)?;

		Ok(Self {
			contents,
			executable,
			references,
		})
	}
}

impl ToV8 for Symlink {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for Symlink {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}

impl ToV8 for symlink::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let object = symlink.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "target".as_bytes()).unwrap();
		let value = self.target.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for symlink::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		if !value.instance_of(scope, symlink.into()).unwrap() {
			return_error!("Expected a symlink.");
		}
		let value = value.to_object(scope).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "target".as_bytes()).unwrap();
		let target = value.get(scope, target.into()).unwrap();
		let target = from_v8(scope, target)?;

		Ok(Self { target })
	}
}

impl ToV8 for Placeholder {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let placeholder =
			v8::String::new_external_onebyte_static(scope, "placeholder".as_bytes()).unwrap();
		let placeholder = tg.get(scope, placeholder.into()).unwrap();
		let placeholder = v8::Local::<v8::Function>::try_from(placeholder).unwrap();

		let object = placeholder.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "name".as_bytes()).unwrap();
		let value = self.name.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for Placeholder {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let placeholder =
			v8::String::new_external_onebyte_static(scope, "Placeholder".as_bytes()).unwrap();
		let placeholder = tg.get(scope, placeholder.into()).unwrap();
		let placeholder = v8::Local::<v8::Function>::try_from(placeholder).unwrap();

		if !value.instance_of(scope, placeholder.into()).unwrap() {
			return_error!("Expected a placeholder.");
		}
		let value = value.to_object(scope).unwrap();

		let name = v8::String::new_external_onebyte_static(scope, "name".as_bytes()).unwrap();
		let name = value.get(scope, name.into()).unwrap();
		let name = from_v8(scope, name)?;

		Ok(Self { name })
	}
}

impl ToV8 for Template {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let template =
			v8::String::new_external_onebyte_static(scope, "Template".as_bytes()).unwrap();
		let template = tg.get(scope, template.into()).unwrap();
		let template = v8::Local::<v8::Function>::try_from(template).unwrap();

		let object = template.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "components".as_bytes()).unwrap();
		let value = self.components.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for Template {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let template =
			v8::String::new_external_onebyte_static(scope, "Template".as_bytes()).unwrap();
		let template = tg.get(scope, template.into()).unwrap();
		let template = v8::Local::<v8::Function>::try_from(template).unwrap();

		if !value.instance_of(scope, template.into()).unwrap() {
			return_error!("Expected a template.");
		}
		let value = value.to_object(scope).unwrap();

		let components =
			v8::String::new_external_onebyte_static(scope, "components".as_bytes()).unwrap();
		let components = value.get(scope, components.into()).unwrap();
		let components = from_v8(scope, components)?;

		Ok(Self { components })
	}
}

impl ToV8 for template::Component {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::String(string) => string.to_v8(scope),
			Self::Artifact(artifact) => artifact.to_v8(scope),
			Self::Placeholder(placeholder) => placeholder.to_v8(scope),
		}
	}
}

impl FromV8 for template::Component {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let artifact =
			v8::String::new_external_onebyte_static(scope, "Artifact".as_bytes()).unwrap();
		let artifact = tg.get(scope, artifact.into()).unwrap();
		let artifact = v8::Local::<v8::Function>::try_from(artifact).unwrap();

		let placeholder =
			v8::String::new_external_onebyte_static(scope, "Placeholder".as_bytes()).unwrap();
		let placeholder = tg.get(scope, placeholder.into()).unwrap();
		let placeholder = v8::Local::<v8::Function>::try_from(placeholder).unwrap();

		let component = if value.is_string() {
			Self::String(from_v8(scope, value)?)
		} else if value.instance_of(scope, artifact.into()).unwrap() {
			Self::Artifact(from_v8(scope, value)?)
		} else if value.instance_of(scope, placeholder.into()).unwrap() {
			Self::Placeholder(from_v8(scope, value)?)
		} else {
			return_error!("Expected a string, artifact, or placeholder.")
		};

		Ok(component)
	}
}

impl ToV8 for Package {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for Package {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}

impl ToV8 for package::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for package::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}

impl ToV8 for Task {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for Task {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}

impl ToV8 for task::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let task = v8::String::new_external_onebyte_static(scope, "Task".as_bytes()).unwrap();
		let task = tg.get(scope, task.into()).unwrap();
		let task = task.to_object(scope).unwrap();
		let task = v8::Local::<v8::Function>::try_from(task).unwrap();

		let object = task.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let value = self.package.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "host".as_bytes()).unwrap();
		let value = self.host.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let value = self.executable.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "target".as_bytes()).unwrap();
		let value = self.target.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let value = self.env.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let value = self.args.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let value = self.checksum.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let value = self.unsafe_.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "network".as_bytes()).unwrap();
		let value = self.network.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for task::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let task = v8::String::new_external_onebyte_static(scope, "Task".as_bytes()).unwrap();
		let task = tg.get(scope, task.into()).unwrap();
		let task = v8::Local::<v8::Function>::try_from(task).unwrap();

		if !value.instance_of(scope, task.into()).unwrap() {
			return_error!("Expected a task.");
		}
		let value = value.to_object(scope).unwrap();

		let package = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let package = value.get(scope, package.into()).unwrap();
		let package = from_v8(scope, package)?;

		let host = v8::String::new_external_onebyte_static(scope, "host".as_bytes()).unwrap();
		let host = value.get(scope, host.into()).unwrap();
		let host = from_v8(scope, host)?;

		let executable =
			v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let executable = value.get(scope, executable.into()).unwrap();
		let executable = from_v8(scope, executable)?;

		let target = v8::String::new_external_onebyte_static(scope, "target".as_bytes()).unwrap();
		let target = value.get(scope, target.into()).unwrap();
		let target = from_v8(scope, target)?;

		let env = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let env = value.get(scope, env.into()).unwrap();
		let env = from_v8(scope, env)?;

		let args = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let args = value.get(scope, args.into()).unwrap();
		let args = from_v8(scope, args)?;

		let checksum =
			v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let checksum = value.get(scope, checksum.into()).unwrap();
		let checksum = from_v8(scope, checksum)?;

		let unsafe_ = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let unsafe_ = value.get(scope, unsafe_.into()).unwrap();
		let unsafe_ = from_v8(scope, unsafe_)?;

		let network = v8::String::new_external_onebyte_static(scope, "network".as_bytes()).unwrap();
		let network = value.get(scope, network.into()).unwrap();
		let network = from_v8(scope, network)?;

		Ok(Self {
			package,
			host,
			executable,
			target,
			env,
			args,
			checksum,
			unsafe_,
			network,
		})
	}
}

impl ToV8 for Checksum {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for Checksum {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		Self::try_from(String::from_v8(scope, value)?)
	}
}

impl ToV8 for System {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for System {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		Self::try_from(String::from_v8(scope, value)?)
	}
}
