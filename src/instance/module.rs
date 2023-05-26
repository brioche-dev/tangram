use crate::{
	document, hash,
	package::{self, Package},
};
use std::collections::HashMap;

/// State required to support module operations.
pub(crate) struct State {

}

impl State {
	pub(crate) fn new() -> State {

		Self {
			packages,
			documents,
		}
	}
}
