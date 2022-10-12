use super::clients;

pub struct Options {
	pub blob_client: Option<clients::blob::Client>,
	pub expression_client: Option<clients::expression::Client>,
}
