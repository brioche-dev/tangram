pub mod login;
pub mod user;

pub struct Client {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Login {
	pub id: Rid,
	pub url: Url,
	pub token: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchResult {
	pub name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
	pub id: Rid,
	pub email: String,
}

impl Client {
	pub async fn create_login(&self) -> Result<Login> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path("/v1/logins");
		let response = self
			.request(reqwest::Method::POST, url)
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let response = response.json().await?;
		Ok(response)
	}
}

impl Client {
	pub async fn get_login(&self, id: Rid) -> Result<Login> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path(&format!("/v1/logins/{id}"));
		let response = self
			.request(reqwest::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let response = response.json().await?;
		Ok(response)
	}
}

impl Client {
	pub async fn publish_package(&self, package: Package) -> Result<()> {
		// Build the URL.
		let id = package.id();
		let mut url = self.url.clone();
		let path = format!("/v1/packages/{id}");
		url.set_path(&path);

		// Send the request.
		self.request(reqwest::Method::POST, url)
			.send()
			.await?
			.error_for_status()?;

		Ok(())
	}
}

impl Client {
	pub async fn search_packages(&self, query: &str) -> Result<Vec<SearchResult>> {
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path("/v1/packages/search");
		url.set_query(Some(&format!("query={query}")));

		// Send the request.
		let response = self
			.request(reqwest::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response.json().await?;

		Ok(response)
	}
}

impl Client {
	pub async fn get_current_user(&self) -> Result<User> {
		// Send the request.
		let mut url = self.url.clone();
		url.set_path("/v1/user");
		let response = self
			.request(reqwest::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Get the response.
		let user = response.json().await?;

		Ok(user)
	}
}
