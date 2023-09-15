use super::{service, Diagnostic};
use crate::{
	error::{return_error, Result},
	module::Module,
	server::Server,
};

impl Module {
	/// Get all diagnostics for the provided modules.
	pub async fn check(server: &Server, modules: Vec<Module>) -> Result<Vec<Diagnostic>> {
		// Create the language service request.
		let request = service::Request::Check(service::check::Request { modules });

		// Handle the language service request.
		let response = server.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Check(response) = response else { return_error!("Unexpected response type.") };

		Ok(response.diagnostics)
	}
}
