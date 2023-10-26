use super::info_string;
use futures::StreamExt;
use ratatui as tui;
use tangram_client as tg;
use tui::prelude::Direction;

pub struct App {
	pub highlighted: usize,
	pub selected: usize,
	pub direction: tui::layout::Direction,
	pub build: Build,
	pub log: Log,
}

pub struct Build {
	pub build: tg::Build,
	pub status: Status,
	pub is_expanded: bool,
	pub children: Vec<Self>,
	pub info: String,
	pub result_receiver: tokio::sync::oneshot::Receiver<tg::Result<tg::Value>>,
	pub children_receiver: tokio::sync::mpsc::UnboundedReceiver<Self>,
}

#[derive(Copy, Clone, Debug)]
pub enum Status {
	InProgress,
	Successful,
	Error,
}

impl App {
	pub fn new(client: &dyn tg::Client, root: tg::Build, root_info: String) -> Self {
		let log = Log::new(client, &root);
		Self {
			highlighted: 0,
			direction: Direction::Horizontal,
			selected: 0,
			build: Build::with_build(client, root, root_info),
			log,
		}
	}

	pub fn select(&mut self, client: &dyn tg::Client) {
		self.selected = self.highlighted;
		let build = self.selected_build();
		self.log = Log::new(client, &build.build);
	}

	pub fn scroll_up(&mut self) {
		self.highlighted = self.highlighted.saturating_sub(1);
	}

	pub fn scroll_down(&mut self) {
		let len = self.build.len() + 1;
		self.highlighted = self
			.highlighted
			.saturating_add(1)
			.min(len.saturating_sub(1));
	}

	pub fn expand(&mut self) {
		let build = self.highlighted_build_mut();
		build.is_expanded = true;
	}

	pub fn collapse(&mut self) {
		let build = self.highlighted_build_mut();
		build.is_expanded = false;
	}

	pub fn rotate(&mut self) {
		self.direction = match self.direction {
			tui::layout::Direction::Horizontal => tui::layout::Direction::Vertical,
			tui::layout::Direction::Vertical => tui::layout::Direction::Horizontal,
		}
	}

	pub fn selected_build(&self) -> &'_ Build {
		self.build.find(self.selected).unwrap()
	}

	fn highlighted_build_mut(&mut self) -> &'_ mut Build {
		self.build.find_mut(self.highlighted).unwrap()
	}
}

impl Build {
	pub fn find(&self, which: usize) -> Option<&'_ Self> {
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

	pub fn with_build(client: &dyn tg::Client, build: tg::Build, info: String) -> Self {
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
			println!("Closing child stream.");
		});

		let (result_sender, result_receiver) = tokio::sync::oneshot::channel();
		let client_ = client.clone_box();
		let build_ = build.clone();
		tokio::task::spawn(async move {
			let result = build_
				.result(client_.as_ref())
				.await
				.and_then(|result| result);
			let _ = result_sender.send(result);
		});

		Self {
			build,
			status: Status::InProgress,
			children: vec![],
			is_expanded: false,
			info,
			children_receiver,
			result_receiver,
		}
	}

	fn len(&self) -> usize {
		self.children
			.iter()
			.fold(self.children.len(), |acc, child| acc + child.len())
	}

	pub fn update(&mut self) {
		if let Ok(child) = self.children_receiver.try_recv() {
			self.children.push(child);
		}
		if let Ok(result) = self.result_receiver.try_recv() {
			self.status = match result {
				Ok(_) => Status::Successful,
				Err(_) => Status::Error,
			}
		}
		self.children.iter_mut().for_each(Self::update);
	}
}

pub struct Log {
	pub text: String,
	receiver: tokio::sync::mpsc::UnboundedReceiver<String>,
	_task: tokio::task::JoinHandle<()>,
}

impl Log {
	pub fn new(client: &dyn tg::Client, build: &tg::Build) -> Self {
		let build = build.clone();
		let client = client.clone_box();
		let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

		let task = tokio::task::spawn(async move {
			let mut log = match build.log(client.as_ref()).await {
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
			text,
			receiver,
			_task: task,
		}
	}

	pub fn update(&mut self) {
		if let Ok(recv) = self.receiver.try_recv() {
			self.text.push_str(recv.as_str());
		}
	}
}
