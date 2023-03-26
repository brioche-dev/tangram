use crate::{error::Result, module};
use futures::{future::LocalBoxFuture, stream::FuturesUnordered};
use sourcemap::SourceMap;
use std::{cell::RefCell, num::NonZeroI32, rc::Rc};

pub struct State {
	pub global_source_map: Option<SourceMap>,
	pub modules: Rc<RefCell<Vec<Module>>>,
	pub futures: Rc<RefCell<FuturesUnordered<LocalBoxFuture<'static, FutureOutput>>>>,
}

#[derive(Debug)]
pub struct Module {
	pub identity_hash: NonZeroI32,
	pub module: v8::Global<v8::Module>,
	pub module_identifier: module::Identifier,
	pub text: String,
	pub transpiled_text: Option<String>,
	pub source_map: Option<SourceMap>,
}

pub struct FutureOutput {
	pub context: v8::Global<v8::Context>,
	pub promise_resolver: v8::Global<v8::PromiseResolver>,
	pub result: Result<v8::Global<v8::Value>>,
}
