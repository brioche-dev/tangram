use crate::artifact::Artifact;

#[derive(Clone, Copy, Debug)]
pub struct Fragment {
	pub(crate) artifact: Artifact,
}

impl Fragment {
	#[must_use]
	pub fn artifact(&self) -> &Artifact {
		&self.artifact
	}
}
