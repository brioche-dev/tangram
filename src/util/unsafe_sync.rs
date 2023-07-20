#[repr(transparent)]
pub struct UnsafeSync<T>(T);

unsafe impl<T> Sync for UnsafeSync<T> {}

impl<T> UnsafeSync<T> {
	pub fn new(value: T) -> Self {
		Self(value)
	}
}

impl<T> std::ops::Deref for UnsafeSync<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> std::ops::DerefMut for UnsafeSync<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
