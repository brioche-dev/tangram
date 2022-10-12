use crate::{expression::Expression, hash::Hash};
use anyhow::Result;
use std::path::Path;

pub struct Db {
	/// This is the LMDB env.
	pub env: lmdb::Environment,

	/// This is the expressions database.
	pub expressions: lmdb::Database,

	/// This is the evaluations database.
	pub evaluations: lmdb::Database,
}

#[derive(buffalo::Serialize, buffalo::Deserialize, serde::Serialize, serde::Deserialize)]
pub struct ExpressionWithOutput {
	#[buffalo(id = 0)]
	pub expression: Expression,
	#[buffalo(id = 1)]
	pub output_hash: Option<Hash>,
}

impl Db {
	pub fn new(path: &Path) -> Result<Db> {
		// Create the env.
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_map_size(1_099_511_627_776);
		env_builder.set_max_dbs(2);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder.open(path)?;

		// Open the expressions database.
		let expressions = env.open_db("expressions".into())?;

		// Open the evaluations database.
		let evaluations = env.open_db("evaluations".into())?;

		// Create the db.
		let db = Db {
			env,
			expressions,
			evaluations,
		};

		Ok(db)
	}
}
