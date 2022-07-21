use super::Client;
use crate::repl::Repl;
use crate::server::graphql::ReplJson;
use anyhow::Result;
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "./src/server/graphql/schema.graphql",
	query_path = "./src/client/repl.graphql"
)]
struct ReplNewMutation;

#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "./src/server/graphql/schema.graphql",
	query_path = "./src/client/repl.graphql"
)]
struct ReplRunMutation;

impl Client {
	pub async fn new_repl(&self) -> Result<Repl> {
		let repl = self
			.request::<ReplNewMutation>(repl_new_mutation::Variables {})
			.await?
			.repl_new;
		let repl = repl.try_into()?;
		Ok(repl)
	}

	pub async fn repl_run(&self, repl: Repl, code: String) -> Result<Option<String>> {
		let repl = repl.try_into()?;
		let output = self
			.request::<ReplRunMutation>(repl_run_mutation::Variables { repl, code })
			.await?
			.repl_run;
		Ok(output)
	}
}
