use crate::{file::File, value::Value};

impl ToV8 for tg::Any {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Value::Null(value) => value.to_v8(scope),
			Value::Bool(value) => value.to_v8(scope),
			Value::Number(value) => value.to_v8(scope),
			Value::String(value) => value.to_v8(scope),
			Value::Bytes(value) => value.to_v8(scope),
			Value::Relpath(value) => value.to_v8(scope),
			Value::Subpath(value) => value.to_v8(scope),
			Value::Blob(value) => value.to_v8(scope),
			Value::Directory(value) => value.to_v8(scope),
			Value::File(value) => value.to_v8(scope),
			Value::Symlink(value) => value.to_v8(scope),
			Value::Placeholder(value) => value.to_v8(scope),
			Value::Template(value) => value.to_v8(scope),
			Value::Package(value) => value.to_v8(scope),
			Value::Resource(value) => value.to_v8(scope),
			Value::Target(value) => value.to_v8(scope),
			Value::Task(value) => value.to_v8(scope),
			Value::Array(value) => value.to_v8(scope),
			Value::Object(value) => value.to_v8(scope),
		}
	}
}

impl FromV8 for tg::Any {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let bytes = v8::String::new_external_onebyte_static(scope, "Bytes".as_bytes()).unwrap();
		let bytes = tg.get(scope, bytes.into()).unwrap();
		let bytes = v8::Local::<v8::Function>::try_from(bytes).unwrap();

		let relpath = v8::String::new_external_onebyte_static(scope, "Relpath".as_bytes()).unwrap();
		let relpath = tg.get(scope, relpath.into()).unwrap();
		let relpath = v8::Local::<v8::Function>::try_from(relpath).unwrap();

		let subpath = v8::String::new_external_onebyte_static(scope, "Subpath".as_bytes()).unwrap();
		let subpath = tg.get(scope, subpath.into()).unwrap();
		let subpath = v8::Local::<v8::Function>::try_from(subpath).unwrap();

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

		let resource =
			v8::String::new_external_onebyte_static(scope, "Resource".as_bytes()).unwrap();
		let resource = tg.get(scope, resource.into()).unwrap();
		let resource = v8::Local::<v8::Function>::try_from(resource).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

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
		} else if value.instance_of(scope, bytes.into()).unwrap() {
			Ok(Value::Bytes(from_v8(scope, value)?))
		} else if value.instance_of(scope, relpath.into()).unwrap() {
			Ok(Value::Relpath(from_v8(scope, value)?))
		} else if value.instance_of(scope, subpath.into()).unwrap() {
			Ok(Value::Subpath(from_v8(scope, value)?))
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
		} else if value.instance_of(scope, resource.into()).unwrap() {
			Ok(Value::Resource(from_v8(scope, value)?))
		} else if value.instance_of(scope, target.into()).unwrap() {
			Ok(Value::Target(from_v8(scope, value)?))
		} else if value.instance_of(scope, task.into()).unwrap() {
			Ok(Value::Task(from_v8(scope, value)?))
		} else if value.is_array() {
			Ok(Value::Array(from_v8(scope, value)?))
		} else if value.is_object() {
			Ok(Value::Object(from_v8(scope, value)?))
		} else {
			return_error!("Invalid value.");
		}
	}
}

impl ToV8 for tg::Bytes {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let value = v8::ArrayBuffer::with_backing_store(scope, &self.buffer().0);
		let value =
			v8::Uint8Array::new(scope, value, self.range().start, self.range().len()).unwrap();
		Ok(value.into())
	}
}

impl FromV8 for tg::Bytes {
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
		let buffer = Buffer(backing_store);
		let range = value.byte_offset()..(value.byte_offset() + value.byte_length());
		Ok(Self { buffer, range })
	}
}

impl ToV8 for tg::Relpath {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::Relpath {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_rust_string_lossy(scope);
		let value = value.parse()?;
		Ok(value)
	}
}

impl ToV8 for tg::Subpath {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::Subpath {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_rust_string_lossy(scope);
		let value = value.parse()?;
		Ok(value)
	}
}

impl ToV8 for tg::Blob {
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

		let key = v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = match &self.kind {
			Kind::Branch(sizes) => sizes.to_v8(scope)?,
			Kind::Leaf(size) => size.to_v8(scope)?,
		};
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Blob {
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

		let kind = v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
		let kind = v8::Private::for_api(scope, Some(kind));
		let kind = value.get_private(scope, kind).unwrap();
		let kind = if kind.is_array() {
			Kind::Branch(from_v8(scope, kind)?)
		} else if kind.is_number() {
			Kind::Leaf(from_v8(scope, kind)?)
		} else {
			return_error!("Expected a kind.");
		};

		Ok(Self { kind })
	}
}

impl ToV8 for tg::Artifact {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Directory(directory) => directory.to_v8(scope),
			Self::File(file) => file.to_v8(scope),
			Self::Symlink(symlink) => symlink.to_v8(scope),
		}
	}
}

impl FromV8 for tg::Artifact {
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

impl ToV8 for tg::Directory {
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
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.entries.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Directory {
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
		let entries = v8::Private::for_api(scope, Some(entries));
		let entries = value.get_private(scope, entries).unwrap();
		let entries = from_v8(scope, entries)?;

		Ok(Self { entries })
	}
}
impl ToV8 for tg::File {
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
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.contents.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.executable.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "references".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.references.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::File {
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
		let contents = v8::Private::for_api(scope, Some(contents));
		let contents = value.get_private(scope, contents).unwrap();
		let contents = from_v8(scope, contents)?;

		let executable =
			v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let executable = v8::Private::for_api(scope, Some(executable));
		let executable = value.get_private(scope, executable).unwrap();
		let executable = from_v8(scope, executable)?;

		let references =
			v8::String::new_external_onebyte_static(scope, "references".as_bytes()).unwrap();
		let references = v8::Private::for_api(scope, Some(references));
		let references = value.get_private(scope, references).unwrap();
		let references = from_v8(scope, references)?;

		Ok(Self {
			contents,
			executable,
			references,
		})
	}
}

impl ToV8 for tg::Symlink {
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
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.target.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Symlink {
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
		let target = v8::Private::for_api(scope, Some(target));
		let target = value.get_private(scope, target).unwrap();
		let target = from_v8(scope, target)?;

		Ok(Self { target })
	}
}

impl ToV8 for tg::Placeholder {
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
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.name.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Placeholder {
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
		let name = v8::Private::for_api(scope, Some(name));
		let name = value.get_private(scope, name).unwrap();
		let name = from_v8(scope, name)?;

		Ok(Self { name })
	}
}

impl ToV8 for tg::Template {
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
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.components.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Template {
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
		let components = v8::Private::for_api(scope, Some(components));
		let components = value.get_private(scope, components).unwrap();
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

impl ToV8 for tg::Build {
	fn to_v8<'a>(
		&self,
		scope: &mut v8::HandleScope<'a>,
	) -> crate::error::Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Resource(resource) => resource.to_v8(scope),
			Self::Target(target) => target.to_v8(scope),
			Self::Task(task) => task.to_v8(scope),
		}
	}
}

impl FromV8 for tg::Build {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> crate::error::Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let resource =
			v8::String::new_external_onebyte_static(scope, "Resource".as_bytes()).unwrap();
		let resource = tg.get(scope, resource.into()).unwrap();
		let resource = v8::Local::<v8::Function>::try_from(resource).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		let task = v8::String::new_external_onebyte_static(scope, "Task".as_bytes()).unwrap();
		let task = tg.get(scope, task.into()).unwrap();
		let task = v8::Local::<v8::Function>::try_from(task).unwrap();

		let operation = if value.instance_of(scope, resource.into()).unwrap() {
			Self::Resource(from_v8(scope, value)?)
		} else if value.instance_of(scope, target.into()).unwrap() {
			Self::Target(from_v8(scope, value)?)
		} else if value.instance_of(scope, task.into()).unwrap() {
			Self::Task(from_v8(scope, value)?)
		} else {
			return_error!("Expected a resource, target, or task.")
		};

		Ok(operation)
	}
}

impl ToV8 for tg::Resource {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		// Get the global.
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let resource =
			v8::String::new_external_onebyte_static(scope, "Resource".as_bytes()).unwrap();
		let resource = tg.get(scope, resource.into()).unwrap();
		let resource = v8::Local::<v8::Function>::try_from(resource).unwrap();

		let object = resource.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "url".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.url.to_string().to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "unpack".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.unpack.map(|unpack| unpack.to_string()).to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.checksum.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.unsafe_.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Resource {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let resource =
			v8::String::new_external_onebyte_static(scope, "Resource".as_bytes()).unwrap();
		let resource = tg.get(scope, resource.into()).unwrap();
		let resource = v8::Local::<v8::Function>::try_from(resource).unwrap();

		if !value.instance_of(scope, resource.into()).unwrap() {
			return_error!("Expected a resource.");
		}
		let value = value.to_object(scope).unwrap();

		let url = v8::String::new_external_onebyte_static(scope, "url".as_bytes()).unwrap();
		let url = v8::Private::for_api(scope, Some(url));
		let url = value.get_private(scope, url).unwrap();
		let url: String = from_v8(scope, url)?;
		let url = Url::parse(&url).map_err(crate::error::Error::other)?;

		let unpack = v8::String::new_external_onebyte_static(scope, "unpack".as_bytes()).unwrap();
		let unpack = v8::Private::for_api(scope, Some(unpack));
		let unpack = value.get_private(scope, unpack).unwrap();
		let unpack: Option<String> = from_v8(scope, unpack)?;
		let unpack = unpack.map(|unpack| unpack.parse().unwrap());

		let checksum =
			v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let checksum = v8::Private::for_api(scope, Some(checksum));
		let checksum = value.get_private(scope, checksum).unwrap();
		let checksum = from_v8(scope, checksum)?;

		let unsafe_ = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let unsafe_ = v8::Private::for_api(scope, Some(unsafe_));
		let unsafe_ = value.get_private(scope, unsafe_).unwrap();
		let unsafe_ = from_v8(scope, unsafe_)?;

		Ok(Self {
			url,
			unpack,
			checksum,
			unsafe_,
		})
	}
}

impl ToV8 for tg::Target {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		let object = target.new_instance(scope, &[]).unwrap();

		let key = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.package.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "path".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.path.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "name_".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.name.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.env.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.args.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Target {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		if !value.instance_of(scope, target.into()).unwrap() {
			return_error!("Expected a target.");
		}
		let value = value.to_object(scope).unwrap();

		let package = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let package = v8::Private::for_api(scope, Some(package));
		let package = value.get_private(scope, package).unwrap();
		let package = from_v8(scope, package)?;

		let path = v8::String::new_external_onebyte_static(scope, "path".as_bytes()).unwrap();
		let path = v8::Private::for_api(scope, Some(path));
		let path = value.get_private(scope, path).unwrap();
		let path = from_v8(scope, path)?;

		let name = v8::String::new_external_onebyte_static(scope, "name_".as_bytes()).unwrap();
		let name = v8::Private::for_api(scope, Some(name));
		let name = value.get_private(scope, name).unwrap();
		let name = from_v8(scope, name)?;

		let env = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let env = v8::Private::for_api(scope, Some(env));
		let env = value.get_private(scope, env).unwrap();
		let env = from_v8(scope, env)?;

		let args = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let args = v8::Private::for_api(scope, Some(args));
		let args = value.get_private(scope, args).unwrap();
		let args = from_v8(scope, args)?;

		Ok(Self {
			package,
			path,
			name,
			env,
			args,
		})
	}
}

impl ToV8 for tg::Task {
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

		let key = v8::String::new_external_onebyte_static(scope, "host".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.host.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.executable.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.env.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.args.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.checksum.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.unsafe_.to_v8(scope)?;
		object.set_private(scope, key, value);

		let key = v8::String::new_external_onebyte_static(scope, "network".as_bytes()).unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.network.to_v8(scope)?;
		object.set_private(scope, key, value);

		Ok(object.into())
	}
}

impl FromV8 for tg::Task {
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

		let host = v8::String::new_external_onebyte_static(scope, "host".as_bytes()).unwrap();
		let host = v8::Private::for_api(scope, Some(host));
		let host = value.get_private(scope, host).unwrap();
		let host = from_v8(scope, host)?;

		let executable =
			v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let executable = v8::Private::for_api(scope, Some(executable));
		let executable = value.get_private(scope, executable).unwrap();
		let executable = from_v8(scope, executable)?;

		let env = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let env = v8::Private::for_api(scope, Some(env));
		let env = value.get_private(scope, env).unwrap();
		let env = from_v8(scope, env)?;

		let args = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let args = v8::Private::for_api(scope, Some(args));
		let args = value.get_private(scope, args).unwrap();
		let args = from_v8(scope, args)?;

		let checksum =
			v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let checksum = v8::Private::for_api(scope, Some(checksum));
		let checksum = value.get_private(scope, checksum).unwrap();
		let checksum = from_v8(scope, checksum)?;

		let unsafe_ = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let unsafe_ = v8::Private::for_api(scope, Some(unsafe_));
		let unsafe_ = value.get_private(scope, unsafe_).unwrap();
		let unsafe_ = from_v8(scope, unsafe_)?;

		let network = v8::String::new_external_onebyte_static(scope, "network".as_bytes()).unwrap();
		let network = v8::Private::for_api(scope, Some(network));
		let network = value.get_private(scope, network).unwrap();
		let network = from_v8(scope, network)?;

		Ok(Self {
			host,
			executable,
			env,
			args,
			checksum,
			unsafe_,
			network,
		})
	}
}
