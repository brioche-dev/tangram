pub struct Builder {
	components: Vec<Component>,
}

impl Builder {
	pub fn push(&mut self, component: Component) {
		self.components.push(component);
	}
}
