use super::{info_string, view::Spinner};
use futures::StreamExt;
use ratatui as tui;
use tangram_client as tg;
use tui::prelude::Direction;

pub struct App {
	/// The rotation of the UI.
	pub direction: tui::layout::Direction,

	/// The info pane.
	pub info: InfoPane,

	/// The build tree.
	pub tree: Tree,
}

/// Model of the state of the build tree view.
pub struct Tree {
	/// The root build of tree.
	pub root: Build,

	/// The number of items in the tree to skip when rendering.
	pub skip: usize,

	/// Which item in the tree is highlighted.
	pub highlighted: usize,
}

/// Model of a single build within the tree.
pub struct Build {
	/// The underlying build object.
	pub build: tg::Build,

	/// If this build should be rendered as expanded.
	pub is_expanded: bool,

	/// A list of the children of this build.
	pub children: Vec<Self>,

	/// An informatic string to display in the tree view.
	pub info: String,

	/// A receiving channel to check for new children.
	pub children_receiver: tokio::sync::mpsc::UnboundedReceiver<Self>,

	/// A receiving channel to check for the result of this build.
	pub result_receiver: tokio::sync::oneshot::Receiver<tg::Result<()>>,

	/// The status of this build.
	pub result: Option<tg::Result<()>>,
}

/// Model of the state of the info pane.
pub enum InfoPane {
	/// A build log.
	Log(Log),

	/// A build result.
	Result(BuildResult),
}

/// The state of a given build log.
pub struct Log {
	/// The underlying build being logged.
	pub build: tg::Build,

	/// The text to display in the view panel.
	pub text: String,

	/// A receiver to poll for new log messages.
	pub receiver: tokio::sync::mpsc::UnboundedReceiver<String>,

	/// Represents the scroll position of the log.
	pub scroll: usize,
}

/// The state of a build's result.
pub struct BuildResult {
	/// The underlying build.
	pub build: tg::Build,

	/// The result of the value, Ok if done, Err if in progress.
	pub value: Option<tg::Result<tg::Value>>,

	/// A receiver to poll for the result of a build.
	pub receiver: tokio::sync::oneshot::Receiver<tg::Result<tg::Value>>,
}

impl App {
	/// Create a new app model for a root build.
	pub fn new(client: &dyn tg::Client, root: tg::Build, root_info: String) -> Self {
		let log = Log::new(client, root.clone());
		Self {
			direction: Direction::Horizontal,
			info: InfoPane::Log(log),
			tree: Tree::new(Build::with_build(client, root, root_info)),
		}
	}

	/// Update any internal state for pending changes.
	pub fn update(&mut self) {
		Spinner::update();
		self.tree.root.update();
		self.info.update();
	}

	/// Select the highlighted build and display its status or log in the info panel.
	pub fn select(&mut self, client: &dyn tg::Client) {
		let build = self.highlighted_build().build.clone();
		match &self.info {
			InfoPane::Log(_) => {
				let log = Log::new(client, build);
				self.info = InfoPane::Log(log);
			},
			InfoPane::Result(_) => {
				let result = BuildResult::new(client, build);
				self.info = InfoPane::Result(result);
			},
		}
	}

	/// Move the highlighted build up one.
	pub fn scroll_up(&mut self) {
		self.tree.highlighted = self.tree.highlighted.saturating_sub(1);
	}

	/// Move the highlighted build down one.
	pub fn scroll_down(&mut self) {
		let len = self.tree.root.len();
		self.tree.highlighted = self.tree.highlighted.saturating_add(1).min(len);
	}

	/// Expand the children of the highlighted build.
	pub fn expand(&mut self) {
		let build = self.highlighted_build_mut();
		build.is_expanded = true;
	}

	/// Collapse the children of the highlighted build.
	pub fn collapse(&mut self) {
		let build = self.highlighted_build_mut();
		build.is_expanded = false;
	}

	/// Rotate the view.
	pub fn rotate(&mut self) {
		self.direction = match self.direction {
			tui::layout::Direction::Horizontal => tui::layout::Direction::Vertical,
			tui::layout::Direction::Vertical => tui::layout::Direction::Horizontal,
		}
	}

	/// Change what is displayed in the info panel.
	pub fn tab_info(&mut self, client: &dyn tg::Client) {
		let build = self.info.build();
		match &self.info {
			InfoPane::Log(_) => {
				let result = BuildResult::new(client, build);
				self.info = InfoPane::Result(result);
			},
			InfoPane::Result(_) => {
				let log = Log::new(client, build);
				self.info = InfoPane::Log(log);
			},
		}
	}

	fn highlighted_build(&self) -> &'_ Build {
		self.tree.root.find(self.tree.highlighted).unwrap()
	}

	fn highlighted_build_mut(&mut self) -> &'_ mut Build {
		self.tree.root.find_mut(self.tree.highlighted).unwrap()
	}
}

impl Tree {
	fn new(root: Build) -> Self {
		Self {
			root,
			skip: 0,
			highlighted: 0,
		}
	}
}

impl Build {
	fn find(&self, which: usize) -> Option<&'_ Self> {
		fn inner(offset: usize, which: usize, build: &'_ Build) -> Result<&'_ Build, usize> {
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
			Err(offset)
		}

		inner(0, which, self).ok()
	}

	fn find_mut(&mut self, which: usize) -> Option<&'_ mut Self> {
		fn inner(
			offset: usize,
			which: usize,
			build: &'_ mut Build,
		) -> Result<&'_ mut Build, usize> {
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
			Err(offset)
		}

		inner(0, which, self).ok()
	}

	fn with_build(client: &dyn tg::Client, build: tg::Build, info: String) -> Self {
		let (children_sender, children_receiver) = tokio::sync::mpsc::unbounded_channel();
		let client_ = client.clone_box();
		let build_ = build.clone();
		tokio::task::spawn(async move {
			let Ok(mut children_stream) = build_.children(client_.as_ref()).await else {
				return;
			};
			while let Some(Ok(child)) = children_stream.next().await {
				let info = info_string(client_.as_ref(), &child).await;
				let child = Self::with_build(client_.as_ref(), child, info);
				if children_sender.send(child).is_err() {
					break;
				}
			}
		});

		let (result_sender, result_receiver) = tokio::sync::oneshot::channel();
		let client_ = client.clone_box();
		let build_ = build.clone();
		tokio::task::spawn(async move {
			let result = build_
				.result(client_.as_ref())
				.await
				.and_then(|r| r)
				.map(|_| ());
			let _ = result_sender.send(result);
		});

		Self {
			build,
			children: vec![],
			is_expanded: false,
			info,
			children_receiver,
			result_receiver,
			result: None,
		}
	}

	fn len(&self) -> usize {
		self.children
			.iter()
			.fold(self.children.len(), |acc, child| acc + child.len())
	}

	fn update(&mut self) {
		if let Ok(child) = self.children_receiver.try_recv() {
			self.children.push(child);
		}

		if self.result.is_none() {
			self.result = match self.result_receiver.try_recv() {
				Ok(status) => Some(status),
				Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
					Some(Err(tg::error!("Failed to get build status.")))
				},
				Err(tokio::sync::oneshot::error::TryRecvError::Empty) => None,
			}
		}

		self.children.iter_mut().for_each(Self::update);
	}
}

impl InfoPane {
	pub fn update(&mut self) {
		match self {
			Self::Log(log) => log.update(),
			Self::Result(result) => result.update(),
		}
	}

	fn build(&self) -> tg::Build {
		match self {
			Self::Log(log) => log.build.clone(),
			Self::Result(result) => result.build.clone(),
		}
	}
}

impl Log {
	fn new(client: &dyn tg::Client, build: tg::Build) -> Self {
		let client = client.clone_box();
		let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

		let build_ = build.clone();
		tokio::task::spawn(async move {
			let mut log = match build_.log(client.as_ref()).await {
				Ok(log) => log,
				Err(e) => {
					let _ = sender.send(e.to_string());
					return;
				},
			};
			while let Some(message) = log.next().await {
				let message = match message.map(|bytes| String::from_utf8(bytes.to_vec())) {
					Ok(Ok(string)) => string,
					Ok(Err(e)) => e.to_string(),
					Err(e) => e.to_string(),
				};
				if sender.send(message).is_err() {
					break;
				}
			}
		});

		let text = String::new();
		Self {
			build,
			text,
			receiver,
			scroll: 0,
		}
	}

	fn update(&mut self) {
		if let Ok(recv) = self.receiver.try_recv() {
			self.text.push_str(recv.as_str());
		}
	}

	pub fn scroll_up(&mut self) {
		self.scroll = self.scroll.saturating_add(1);
	}

	pub fn scroll_down(&mut self) {
		self.scroll = self.scroll.saturating_sub(1);
	}
}

impl BuildResult {
	fn new(client: &dyn tg::Client, build: tg::Build) -> Self {
		let (sender, receiver) = tokio::sync::oneshot::channel();
		let value = None;
		let client = client.clone_box();
		let build_ = build.clone();
		let _task = tokio::task::spawn(async move {
			let value = build_.result(client.as_ref()).await.and_then(|r| r);
			let _ = sender.send(value);
		});
		Self {
			build,
			value,
			receiver,
		}
	}

	fn update(&mut self) {
		if self.value.is_some() {
			return;
		}

		self.value = match self.receiver.try_recv() {
			Ok(value) => Some(value),
			Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
				Some(Err(tg::error!("Failed to get value for build.")))
			},
			Err(tokio::sync::oneshot::error::TryRecvError::Empty) => None,
		}
	}
}
