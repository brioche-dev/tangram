use ratatui as tui;
use tangram_client as tg;

use super::event_stream::EventStream;

pub struct App {
	pub selected: usize,
	pub direction: tui::layout::Direction,
	pub log: String,
	pub builds: Vec<Build>,
}

pub struct Build {
	pub build: tg::Build,
	pub status: Status,
	pub is_expanded: bool,
	pub children: Vec<Self>,
}

#[derive(Copy, Clone, Debug)]
pub enum Status {
	InProgress,
	Successful,
	Error,
}

impl App {
	fn select_log(&mut self, client: &dyn tg::Client, event_stream: &EventStream) {
		let build = self.selected_build().build.clone();
		self.log.clear();
		event_stream.set_log(build)
	}

	pub fn scroll_up(&mut self) {
		self.selected = self.selected.saturating_sub(1);
	}

	pub fn scroll_down(&mut self) {
		let len = self
			.builds
			.iter()
			.fold(self.builds.len(), |acc, build| acc + build.len());
		self.selected = self.selected.saturating_add(1).min(len.saturating_sub(1));
	}

	pub fn expand(&mut self, client: &dyn tg::Client) {
		let build = self.selected_build_mut();
		build.is_expanded = true;
	}

	pub fn collapse(&mut self) {
		let build = self.selected_build_mut();
		build.is_expanded = true;
		build.is_expanded = false;
	}

	pub fn rotate(&mut self) {
		self.direction = match self.direction {
			tui::layout::Direction::Horizontal => tui::layout::Direction::Vertical,
			tui::layout::Direction::Vertical => tui::layout::Direction::Horizontal,
		}
	}

	fn build_by_id_mut(&self, id: tg::build::Id) -> Option<&'_ mut Build> {
		find_build_by_id_mut(&mut self.builds, id)
	}

	fn selected_build(&self) -> &'_ Build {
		find_build(&self.builds, self.selected).unwrap()
	}

	fn selected_build_mut(&mut self) -> &'_ mut Build {
		find_build_mut(&mut self.builds, self.selected).unwrap()
	}
}

fn find_build<'a>(builds: &'a [Build], which: usize) -> Option<&'a Build> {
	fn inner<'a>(offset: usize, which: usize, build: &'a Build) -> Result<&'a Build, usize> {
		if offset == which {
			return Ok(build);
		}
		let mut offset = offset + 1;
		if !build.is_expanded {
			return Err(offset);
		}
		for child in &build.children {
			match inner(offset, which, child) {
				Ok(found) => return Ok(found),
				Err(o) => offset = o,
			}
		}
		return Err(offset);
	}
	let mut offset = 0;
	for build in builds {
		match inner(offset, which, build) {
			Ok(found) => return Some(found),
			Err(o) => offset = o,
		}
	}
	None
}

fn find_build_mut<'a>(builds: &'a mut [Build], which: usize) -> Option<&'a mut Build> {
	fn inner<'a>(
		offset: usize,
		which: usize,
		build: &'a mut Build,
	) -> Result<&'a mut Build, usize> {
		if offset == which {
			return Ok(build);
		}
		let mut offset = offset + 1;
		if !build.is_expanded {
			return Err(offset);
		}
		for child in &mut build.children {
			match inner(offset, which, child) {
				Ok(found) => return Ok(found),
				Err(o) => offset = o,
			}
		}
		return Err(offset);
	}
	let mut offset = 0;
	for build in builds {
		match inner(offset, which, build) {
			Ok(found) => return Some(found),
			Err(o) => offset = o,
		}
	}
	None
}

fn find_build_by_id_mut<'a>(builds: &'a mut [Build], id: tg::build::Id) -> Option<&'a mut Build> {
	fn inner<'a>(id: tg::build::Id, build: &'a mut Build) -> Option<&'a mut Build> {
		if build.build.id() == id {
			return Some(build);
		}
		for child in &mut build.children {
			if let Some(found) = inner(id, child) {
				return Some(found);
			}
		}
		None
	}
	for build in builds {
		if let Some(found) = inner(id, build) {
			return Some(found);
		}
	}
	None
}

impl Build {
	fn len(&self) -> usize {
		self.children
			.iter()
			.fold(self.children.len(), |acc, child| acc + child.len())
	}
}
