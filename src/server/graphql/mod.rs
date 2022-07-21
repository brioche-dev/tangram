pub use self::{mutation::Mutation, query::Query, scalars::*};
use crate::server::Server;
use std::sync::Arc;

mod mutation;
mod query;
mod scalars;

pub type Schema =
	juniper::RootNode<'static, Query, Mutation, juniper::EmptySubscription<Arc<Server>>>;
