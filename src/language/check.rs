use super::{service, Diagnostic};
use crate::{
	error::{bail, Result},
	module, Instance,
};
use std::{collections::BTreeMap, sync::Arc};

impl Instance {
	/// Get all diagnostics for the provided module identifiers.
	pub async fn check(
		self: &Arc<Self>,
		module_identifiers: Vec<module::Identifier>,
	) -> Result<BTreeMap<module::Identifier, Vec<Diagnostic>>> {
		// Create the language service request.
		let request = service::Request::Check(service::check::Request { module_identifiers });

		// Send the language service request and receive the response.
		let response = self.language_service_request(request).await?;

		// Get the response.
		let service::Response::Check(response) = response else { bail!("Unexpected response type.") };

		Ok(response.diagnostics)
	}
}
