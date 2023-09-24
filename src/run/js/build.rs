use super::{from_v8, isolate::THREAD_LOCAL_ISOLATE, state::State, Target, ToV8};
use crate::{
	id::Id,
	module::{self, Module},
	server::Server,
	value::Value,
	Error, Result, WrapErr,
};
use std::rc::Rc;

#[derive(serde::Serialize)]
struct Key {
	package: Id,
	path: String,
	name: String,
}

impl Target {
	/// Build the target.
	#[tracing::instrument(skip(tg), ret)]
	pub async fn build(&self, server: &Server) -> Result<Value> {
		let operation = Build::Target(self.clone());
		operation.output(tg, None).await
	}

	pub(crate) async fn build_inner(&self, server: &Server) -> Result<Value> {
		// Build the target on the server's local pool because it is a `!Send` future.
		let output = tg
			.local_pool
			.spawn_pinned({
				let tg = tg.clone();
				let target = self.clone();
				move || async move { target.build_inner_inner(tg).await }
			})
			.await
			.map_err(Error::other)
			.wrap_err("Failed to join the task.")??;

		Ok(output)
	}

	#[allow(clippy::await_holding_refcell_ref, clippy::too_many_lines)]
	async fn build_inner_inner(&self, tg: Server) -> Result<Value> {
		// Create the context.
		let context = super::context::new(tg.clone());

		// Evaluate the module.
		let module = Module::Normal(module::Normal {
			package: self.package.id(),
			module_path: self.path.clone(),
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
		let tg =
			v8::String::new_external_onebyte_static(&mut try_catch_scope, "tg".as_bytes()).unwrap();
		let tg = global.get(&mut try_catch_scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		// Get the targets.
		let targets =
			v8::String::new_external_onebyte_static(&mut try_catch_scope, "targets".as_bytes())
				.unwrap();
		let targets = tg.get(&mut try_catch_scope, targets.into()).unwrap();
		let targets = v8::Local::<v8::Object>::try_from(targets).unwrap();

		// Get the target function.
		let (package, path) = match &module {
			Module::Normal(module) => (module.package, module.module_path.to_string()),
			_ => unreachable!(),
		};
		let key = Key {
			package,
			path,
			name: self.name.clone(),
		};
		let key = serde_json::to_string(&key).unwrap();
		let key = serde_v8::to_v8(&mut try_catch_scope, key).map_err(Error::other)?;
		let function = targets
			.get(&mut try_catch_scope, key)
			.wrap_err("Failed to get the target function.")?;
		let function = v8::Local::<v8::Function>::try_from(function)
			.map_err(Error::other)
			.wrap_err("Expected a function.")?;

		// Move the env to v8.
		let env = self
			.env
			.to_v8(&mut try_catch_scope)
			.wrap_err("Failed to move the env to v8.")?;

		// Set the env.
		let env_object =
			v8::String::new_external_onebyte_static(&mut try_catch_scope, "env".as_bytes())
				.unwrap();
		let env_object = tg.get(&mut try_catch_scope, env_object.into()).unwrap();
		let env_object = v8::Local::<v8::Object>::try_from(env_object).unwrap();
		let value =
			v8::String::new_external_onebyte_static(&mut try_catch_scope, "value".as_bytes())
				.unwrap();
		env_object.set(&mut try_catch_scope, value.into(), env);

		// Move the args to v8.
		let args = self
			.args
			.iter()
			.map(|arg| arg.to_v8(&mut try_catch_scope))
			.collect::<Result<Vec<_>>>()
			.wrap_err("Failed to move the args to v8.")?;

		// Call the function.
		let output = function.call(&mut try_catch_scope, undefined.into(), &args);
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

		// Move the output from v8.
		let output = from_v8(&mut context_scope, output)?;

		// Exit the context.
		drop(context_scope);
		drop(handle_scope);
		drop(isolate);

		Ok(output)
	}
}
