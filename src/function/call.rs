use super::{isolate::THREAD_LOCAL_ISOLATE, state::State, Function};
use crate::{
	error::{Error, Result, WrapErr},
	instance::Instance,
	module::{self, Module},
	operation::Operation,
	value::Value,
};
use std::{rc::Rc, sync::Arc};

impl Function {
	/// Call the function.
	#[tracing::instrument(skip(tg), ret)]
	pub async fn call(&self, tg: &Arc<Instance>) -> Result<Value> {
		let operation = Operation::Function(self.clone());
		operation.run(tg).await
	}

	pub(crate) async fn call_inner(&self, tg: &Arc<Instance>) -> Result<Value> {
		// Call the function on the local pool because it is a `!Send` future.
		let output = tg
			.language
			.local_pool
			.spawn_pinned({
				let tg = tg.clone();
				let function = self.clone();
				move || async move { function.call_inner_inner(tg).await }
			})
			.await
			.map_err(Error::other)
			.wrap_err("Failed to join the task.")??;

		Ok(output)
	}

	#[allow(
		clippy::await_holding_refcell_ref,
		clippy::too_many_lines,
		clippy::items_after_statements
	)]
	async fn call_inner_inner(&self, tg: Arc<Instance>) -> Result<Value> {
		// Create the context.
		let context = super::context::new(tg.clone());

		// Evaluate the module.
		let module = Module::Normal(module::Normal {
			package_hash: self.package_hash,
			module_path: self.module_path.clone(),
		});
		super::module::evaluate(context.clone(), &module).await?;

		// Enter the context.
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Get the state.
		let state = context
			.get_slot::<Rc<State>>(&mut context_scope)
			.unwrap()
			.clone();

		// Create a try catch scope.
		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
		let undefined = v8::undefined(&mut try_catch_scope);

		// Get the tg global.
		let global = context.global(&mut try_catch_scope);
		let tg_string = v8::String::new(&mut try_catch_scope, "tg").unwrap();
		let tg: v8::Local<v8::Object> = global
			.get(&mut try_catch_scope, tg_string.into())
			.unwrap()
			.try_into()
			.unwrap();

		// Get the registry.
		let registry_string = v8::String::new(&mut try_catch_scope, "registry").unwrap();
		let registry: v8::Local<v8::Object> = tg
			.get(&mut try_catch_scope, registry_string.into())
			.unwrap()
			.try_into()
			.unwrap();

		// Get the function.
		#[derive(serde::Serialize)]
		struct RegistryKey {
			module: Module,
			name: String,
		}
		let key = RegistryKey {
			module,
			name: self.name.clone(),
		};
		let key = serde_json::to_value(&key).unwrap();
		let key = serde_json::to_string(&key).unwrap();
		let key = serde_v8::to_v8(&mut try_catch_scope, key).map_err(Error::other)?;
		let function: v8::Local<v8::Function> = registry
			.get(&mut try_catch_scope, key)
			.wrap_err("Failed to get the function.")?
			.try_into()
			.map_err(Error::other)
			.wrap_err("Expected a function.")?;

		// Get the implementation.
		let f_string = v8::String::new(&mut try_catch_scope, "f").unwrap();
		let f: v8::Local<v8::Function> = function
			.get(&mut try_catch_scope, f_string.into())
			.wrap_err(r#"Failed to find a value for the key "f"."#)?
			.try_into()
			.map_err(Error::other)
			.wrap_err(r#"The value for the key "f" must be a function."#)?;

		// Get the entrypoint.
		let entrypoint_string = v8::String::new(&mut try_catch_scope, "entrypoint").unwrap();
		let entrypoint: v8::Local<v8::Function> = tg
			.get(&mut try_catch_scope, entrypoint_string.into())
			.unwrap()
			.try_into()
			.unwrap();

		// Serialize the env to v8.
		let env = serde_v8::to_v8(&mut try_catch_scope, &self.env)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the env.")?;

		// Serialize the args to v8.
		let args = serde_v8::to_v8(&mut try_catch_scope, &self.args)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the args.")?;

		// Call the entrypoint.
		let output = entrypoint.call(
			&mut try_catch_scope,
			undefined.into(),
			&[f.into(), env, args],
		);
		let Some(output) = output else {
			let exception = try_catch_scope.exception().unwrap();
			let error = Error::from_exception(&mut try_catch_scope, &state, exception);
			return Err(error);
		};

		// Make the output and context global.
		let output = v8::Global::new(&mut try_catch_scope, output);
		let context = v8::Global::new(&mut try_catch_scope, context);

		// Exit the context.
		drop(try_catch_scope);
		drop(context_scope);
		drop(handle_scope);
		drop(isolate);

		// Await the output.
		let output = super::context::await_value(context.clone(), output).await?;

		// Enter the context.
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Move the output to the context.
		let output = v8::Local::new(&mut context_scope, output);

		// Deserialize the output from v8.
		let output = serde_v8::from_v8(&mut context_scope, output).map_err(Error::other)?;

		// Exit the context.
		drop(context_scope);
		drop(handle_scope);
		drop(isolate);

		Ok(output)
	}
}
