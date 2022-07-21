use super::scalars::ReplJson;
use crate::server::Server;
use std::sync::Arc;

pub struct Mutation;

#[juniper::graphql_object(context = Arc<Server>)]
impl Mutation {
	async fn repl_new(server: &Arc<Server>) -> juniper::FieldResult<ReplJson> {
		let repl = server.repl_new().await?;
		let repl = repl.try_into()?;
		Ok(repl)
	}

	async fn repl_run(
		server: &Arc<Server>,
		repl: ReplJson,
		code: String,
	) -> juniper::FieldResult<Option<String>> {
		let repl = repl.try_into()?;
		let output = server.repl_run(&repl, code).await?;
		Ok(output)
	}
}
