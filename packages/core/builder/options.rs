use super::clients;

pub struct Options {
	pub blob_client: clients::blob::Client,
	pub expression_client: clients::expression::Client,
}
