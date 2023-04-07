use super::{Hash, Instance};
use crate::error::{Result, WrapErr};

impl Instance {
	pub async fn get(tg: &crate::instance::Instance, hash: Hash) -> Result<Self> {
		let package_instance = Self::try_get(tg, hash).await?.wrap_err_with(|| {
			format!(r#"Failed to get the package instance with hash "{hash}"."#)
		})?;
		Ok(package_instance)
	}

	pub async fn try_get(tg: &crate::instance::Instance, hash: Hash) -> Result<Option<Self>> {
		// Attempt to get the package instance from the database.
		if let Some(package_instance) = Self::try_get_local(tg, hash).await? {
			return Ok(Some(package_instance));
		}

		// // Attempt to get the package instance from the API.
		// let package_instance = tg
		// 	.api_instance_client()
		// 	.try_get_package_instance(hash)
		// 	.await
		// 	.ok()
		// 	.flatten();
		// if let Some(package_instance) = package_instance {
		// 	return Ok(Some(package_instance));
		// }

		Ok(None)
	}

	pub async fn get_local(tg: &crate::instance::Instance, hash: Hash) -> Result<Self> {
		let package_instance = Self::try_get_local(tg, hash).await?.wrap_err_with(|| {
			format!(r#"Failed to find the package instance with hash "{hash}"."#)
		})?;
		Ok(package_instance)
	}

	pub async fn try_get_local(tg: &crate::instance::Instance, hash: Hash) -> Result<Option<Self>> {
		// Get the serialized package instance from the database.
		let Some(package_instance) = tg.database.try_get_package_instance(hash).await? else {
			return Ok(None);
		};

		// Create the package instance from the serialized package instance.
		let package_instance = Self::from_data(tg, hash, package_instance).await?;

		Ok(Some(package_instance))
	}
}
