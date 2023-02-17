use crate::artifact;

mod add;
mod get;
mod serialize;

#[derive(buffalo::Serialize, buffalo::Deserialize)]
pub struct Tracker {
	#[buffalo(id = 0)]
	pub artifact_hash: artifact::Hash,
	#[buffalo(id = 1)]
	pub timestamp_seconds: u64,
	#[buffalo(id = 2)]
	pub timestamp_nanoseconds: u32,
}
