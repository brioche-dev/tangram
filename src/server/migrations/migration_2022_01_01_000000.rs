use anyhow::Result;
use async_trait::async_trait;
use sqlx::prelude::*;

pub struct Migration;

#[async_trait]
impl super::Migration for Migration {
	async fn run(&self, txn: &mut sqlx::Transaction<sqlx::Sqlite>) -> Result<()> {
		txn.execute(include_str!("./migration_2022_01_01_000000.sql"))
			.await?;
		Ok(())
	}
}
