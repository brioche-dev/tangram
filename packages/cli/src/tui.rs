use crossterm as ct;
use futures::StreamExt;
use num::ToPrimitive;
use ratatui as tui;
use std::{
	collections::VecDeque,
	sync::{
		atomic::{AtomicBool, AtomicUsize},
		Arc,
	},
};
use tangram_client as tg;
use tangram_error::{Result, WrapErr};
use tangram_package::PackageExt;

type Backend = tui::backend::CrosstermBackend<std::fs::File>;

type Terminal = tui::Terminal<Backend>;

type Frame<'a> = tui::Frame<'a, Backend>;

pub struct Tui {
	#[allow(dead_code)]
	options: Options,
	stop: Arc<AtomicBool>,
	task: Option<tokio::task::JoinHandle<std::io::Result<Terminal>>>,
}

struct App {
	client: Box<dyn tg::Client>,
	direction: tui::layout::Direction,
	tree: Tree,
	log: Log,
}

struct Tree {
	root: TreeItem,
	scroll: usize,
	selected: usize,
}

struct TreeItem {
	client: Box<dyn tg::Client>,
	build: tg::Build,
	depth: usize,
	last: bool,
	selected: bool,
	expanded: bool,
	status: TreeItemStatus,
	title: Option<String>,
	children: Vec<Self>,
	status_receiver: tokio::sync::oneshot::Receiver<TreeItemStatus>,
	title_receiver: tokio::sync::oneshot::Receiver<Option<String>>,
	children_receiver: tokio::sync::mpsc::UnboundedReceiver<tg::Build>,
}

enum TreeItemStatus {
	Building,
	Failure,
	Success,
}

struct Log {
	scroll: usize,
	text: String,
	receiver: tokio::sync::mpsc::UnboundedReceiver<String>,
}

static SPINNER_POSITION: AtomicUsize = AtomicUsize::new(0);
const SPINNER: [&str; 8] = ["|", "/", "-", "\\", "|", "/", "-", "\\"];
const SPINNER_FRAMES_PER_UPDATE: usize = 8;

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub exit: bool,
}

impl Tui {
	pub async fn start(
		client: &dyn tg::Client,
		build: &tg::Build,
		options: Options,
	) -> Result<Self> {
		// Create the terminal.
		let tty = tokio::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.open("/dev/tty")
			.await
			.wrap_err("Failed to open /dev/tty.")?;
		let tty = tty.into_std().await;
		let backend = Backend::new(tty);
		let mut terminal =
			Terminal::new(backend).wrap_err("Failed to create the terminal backend.")?;
		ct::terminal::enable_raw_mode().wrap_err("Failed to enable the terminal's raw mode")?;
		ct::execute!(
			terminal.backend_mut(),
			ct::event::EnableMouseCapture,
			ct::terminal::EnterAlternateScreen,
		)
		.wrap_err("Failed to setup the terminal.")?;

		// Create the stop flag.
		let stop = Arc::new(AtomicBool::new(false));

		// Spawn the task.
		let task = tokio::task::spawn_blocking({
			let client = client.clone_box();
			let build = build.clone();
			let stop = stop.clone();
			move || {
				let mut app = App::new(client.as_ref(), &build);
				while !stop.load(std::sync::atomic::Ordering::SeqCst) {
					if ct::event::poll(std::time::Duration::from_millis(16))? {
						let event = ct::event::read()?;
						if let ct::event::Event::Key(event) = event {
							if event.code == ct::event::KeyCode::Char('q') && options.exit {
								break;
							}
						}
						app.handle_event(&event);
					}
					app.update();
					terminal.draw(|frame| app.render(frame, frame.size()))?;
				}
				Ok(terminal)
			}
		});

		Ok(Self {
			options,
			stop,
			task: Some(task),
		})
	}

	pub fn stop(&self) {
		// Set the stop flag.
		let ordering = std::sync::atomic::Ordering::SeqCst;
		self.stop.store(true, ordering);
	}

	pub async fn join(mut self) -> Result<()> {
		// Get the task.
		let Some(task) = self.task.take() else {
			return Ok(());
		};

		// Join the task and get the terminal.
		let mut terminal = task.await.unwrap().wrap_err("The task did not succeed.")?;

		// Reset the terminal.
		terminal.clear().wrap_err("Failed to clear the terminal.")?;
		ct::execute!(
			terminal.backend_mut(),
			ct::event::DisableMouseCapture,
			ct::terminal::LeaveAlternateScreen
		)
		.wrap_err("Failed to reset the terminal.")?;
		ct::terminal::disable_raw_mode().wrap_err("Failed to disable the terminal's raw mode.")?;

		Ok(())
	}
}

impl App {
	fn new(client: &dyn tg::Client, root: &tg::Build) -> Self {
		let client = client.clone_box();
		let direction = tui::layout::Direction::Horizontal;
		let tree = Tree::new(TreeItem::new(client.as_ref(), root, 0, true, true, true));
		let log = Log::new(client.as_ref(), root);
		Self {
			client,
			direction,
			tree,
			log,
		}
	}

	fn handle_event(&mut self, event: &ct::event::Event) {
		match event {
			ct::event::Event::Key(event) => self.handle_key_event(*event),
			ct::event::Event::Mouse(event) => self.handle_mouse_event(*event),
			_ => (),
		}
	}

	fn handle_key_event(&mut self, event: ct::event::KeyEvent) {
		match event.code {
			ct::event::KeyCode::Left | ct::event::KeyCode::Char('h') => {
				self.collapse();
			},
			ct::event::KeyCode::Down | ct::event::KeyCode::Char('j') => {
				self.down();
			},
			ct::event::KeyCode::Up | ct::event::KeyCode::Char('k') => {
				self.up();
			},
			ct::event::KeyCode::Right | ct::event::KeyCode::Char('l') => {
				self.expand();
			},
			ct::event::KeyCode::Char('r') => {
				self.rotate();
			},
			_ => (),
		}
	}

	fn handle_mouse_event(&mut self, event: ct::event::MouseEvent) {
		match event.kind {
			ct::event::MouseEventKind::ScrollDown => {
				self.log.scroll_down();
			},
			ct::event::MouseEventKind::ScrollUp => {
				self.log.scroll_up();
			},
			_ => (),
		}
	}

	fn update(&mut self) {
		SPINNER_POSITION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		self.tree.update();
		self.log.update();
	}

	fn down(&mut self) {
		self.select(
			self.tree
				.selected
				.saturating_add(1)
				.min(self.tree.visible_items_count()),
		);
	}

	fn up(&mut self) {
		self.select(self.tree.selected.saturating_sub(1));
	}

	fn select(&mut self, index: usize) {
		let selected_item = self.tree.selected_item_mut();
		selected_item.selected = false;
		self.tree.selected = index;
		let selected_item = self.tree.selected_item_mut();
		selected_item.selected = true;
		self.log = Log::new(self.client.as_ref(), &selected_item.build);
	}

	fn expand(&mut self) {
		let item = self.tree.selected_item_mut();
		item.expanded = true;
	}

	fn collapse(&mut self) {
		let item = self.tree.selected_item_mut();
		item.expanded = false;
	}

	fn rotate(&mut self) {
		self.direction = match self.direction {
			tui::layout::Direction::Horizontal => tui::layout::Direction::Vertical,
			tui::layout::Direction::Vertical => tui::layout::Direction::Horizontal,
		}
	}

	fn render(&self, frame: &mut Frame, area: tui::layout::Rect) {
		let layout = tui::layout::Layout::default()
			.direction(self.direction)
			.margin(0)
			.constraints([
				tui::layout::Constraint::Percentage(50),
				tui::layout::Constraint::Length(1),
				tui::layout::Constraint::Min(1),
			])
			.split(area);

		self.tree.render(frame, layout[0]);

		let border = match self.direction {
			tui::layout::Direction::Horizontal => tui::widgets::Borders::LEFT,
			tui::layout::Direction::Vertical => tui::widgets::Borders::BOTTOM,
		};
		let block = tui::widgets::Block::default().borders(border);
		frame.render_widget(block, layout[1]);

		self.log.render(frame, layout[2]);
	}
}

impl Tree {
	fn new(root: TreeItem) -> Self {
		Self {
			root,
			scroll: 0,
			selected: 0,
		}
	}

	fn _selected_item(&self) -> &'_ TreeItem {
		fn inner(item: &'_ TreeItem, index: usize, selected: usize) -> Result<&'_ TreeItem, usize> {
			if index == selected {
				return Ok(item);
			}
			let mut index = index + 1;
			if !item.expanded {
				return Err(index);
			}
			for child in &item.children {
				match inner(child, index, selected) {
					Ok(item) => return Ok(item),
					Err(i) => index = i,
				}
			}
			Err(index)
		}
		inner(&self.root, 0, self.selected).unwrap()
	}

	fn selected_item_mut(&mut self) -> &'_ mut TreeItem {
		fn inner(
			item: &'_ mut TreeItem,
			index: usize,
			selected: usize,
		) -> Result<&'_ mut TreeItem, usize> {
			if index == selected {
				return Ok(item);
			}
			let mut index = index + 1;
			if !item.expanded {
				return Err(index);
			}
			for child in &mut item.children {
				match inner(child, index, selected) {
					Ok(item) => return Ok(item),
					Err(i) => index = i,
				}
			}
			Err(index)
		}
		inner(&mut self.root, 0, self.selected).unwrap()
	}

	fn visible_items_count(&self) -> usize {
		fn inner(item: &'_ TreeItem) -> usize {
			let mut count = 1;
			if item.expanded {
				for child in &item.children {
					count += inner(child);
				}
			}
			count
		}
		inner(&self.root)
	}

	fn update(&mut self) {
		self.root.update();
	}

	fn render(&self, frame: &mut Frame, area: tui::layout::Rect) {
		let layout = tui::layout::Layout::default()
			.direction(tui::layout::Direction::Vertical)
			.constraints(
				(0..area.height)
					.map(|_| tui::layout::Constraint::Length(1))
					.collect::<Vec<_>>(),
			)
			.split(area);
		let mut stack = VecDeque::from(vec![&self.root]);
		let mut index = 0;
		while let Some(item) = stack.pop_front() {
			if item.expanded {
				for child in item.children.iter().rev() {
					stack.push_front(child);
				}
			}
			if index >= self.scroll && index < self.scroll + area.height.to_usize().unwrap() {
				item.render(frame, layout[index - self.scroll]);
			}
			index += 1;
		}
	}
}

impl TreeItem {
	fn new(
		client: &dyn tg::Client,
		build: &tg::Build,
		depth: usize,
		last: bool,
		selected: bool,
		expanded: bool,
	) -> Self {
		let (status_sender, status_receiver) = tokio::sync::oneshot::channel();
		tokio::task::spawn({
			let client = client.clone_box();
			let build = build.clone();
			async move {
				let status = match build.result(client.as_ref()).await {
					Err(_) | Ok(Err(_)) => TreeItemStatus::Failure,
					Ok(Ok(_)) => TreeItemStatus::Success,
				};
				status_sender.send(status).ok();
			}
		});

		let (title_sender, title_receiver) = tokio::sync::oneshot::channel();
		tokio::task::spawn({
			let client = client.clone_box();
			let build = build.clone();
			async move {
				let title = title(client.as_ref(), &build).await.ok().flatten();
				title_sender.send(title).ok();
			}
		});

		let (children_sender, children_receiver) = tokio::sync::mpsc::unbounded_channel();
		tokio::task::spawn({
			let client = client.clone_box();
			let build = build.clone();
			async move {
				let Ok(mut children) = build.children(client.as_ref()).await else {
					return;
				};
				while let Some(Ok(child)) = children.next().await {
					let result = children_sender.send(child);
					if result.is_err() {
						break;
					}
				}
			}
		});

		Self {
			client: client.clone_box(),
			build: build.clone(),
			depth,
			last,
			selected,
			expanded,
			status: TreeItemStatus::Building,
			title: None,
			children: Vec::new(),
			status_receiver,
			title_receiver,
			children_receiver,
		}
	}

	fn update(&mut self) {
		if let Ok(status) = self.status_receiver.try_recv() {
			self.status = status;
		}
		if let Ok(title) = self.title_receiver.try_recv() {
			self.title = title;
		}
		while let Ok(child) = self.children_receiver.try_recv() {
			if let Some(child) = self.children.last_mut() {
				child.last = false;
			}
			let child = TreeItem::new(
				self.client.as_ref(),
				&child,
				self.depth + 1,
				true,
				false,
				false,
			);
			self.children.push(child);
		}
		for child in &mut self.children {
			child.update();
		}
	}

	fn render(&self, frame: &mut Frame, area: tui::layout::Rect) {
		let mut prefix = String::new();
		for _ in 0..self.depth.saturating_sub(1) {
			prefix.push_str("│ ");
		}
		if self.depth > 0 {
			prefix.push_str(if self.last { "└─" } else { "├─" });
		}
		let disclosure = if self.expanded { "▼" } else { "▶" };
		let status = match self.status {
			TreeItemStatus::Building => {
				let state = SPINNER_POSITION.load(std::sync::atomic::Ordering::SeqCst);
				let state = (state / SPINNER_FRAMES_PER_UPDATE) % SPINNER.len();
				SPINNER[state]
			},
			TreeItemStatus::Failure => "✗",
			TreeItemStatus::Success => "✓",
		};
		let title = self.title.as_deref().unwrap_or("<unknown>");
		let title = tui::text::Text::from(format!("{prefix}{disclosure} {status} {title}"));
		let style = if self.selected {
			tui::style::Style::default()
				.bg(tui::style::Color::White)
				.fg(tui::style::Color::Black)
		} else {
			tui::style::Style::default()
		};
		frame.render_widget(tui::widgets::Paragraph::new(title).style(style), area);
	}
}

impl Log {
	fn new(client: &dyn tg::Client, build: &tg::Build) -> Self {
		let client = client.clone_box();
		let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

		tokio::task::spawn({
			let build = build.clone();
			async move {
				let mut log = match build.log(client.as_ref()).await {
					Ok(log) => log,
					Err(error) => {
						sender.send(error.to_string()).ok();
						return;
					},
				};
				while let Some(message) = log.next().await {
					let message = match message.map(|bytes| String::from_utf8(bytes.to_vec())) {
						Ok(Ok(string)) => string,
						Ok(Err(error)) => error.to_string(),
						Err(error) => error.to_string(),
					};
					if sender.send(message).is_err() {
						break;
					}
				}
			}
		});

		let text = String::new();
		Self {
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

	fn scroll_down(&mut self) {
		self.scroll = self.scroll.saturating_add(1);
	}

	fn scroll_up(&mut self) {
		self.scroll = self.scroll.saturating_sub(1);
	}

	fn render(&self, frame: &mut Frame, area: tui::layout::Rect) {
		let text = tui::text::Text::from(self.text.as_str());
		let wrap = tui::widgets::Wrap { trim: false };
		let widget = tui::widgets::Paragraph::new(text)
			.wrap(wrap)
			.scroll((self.scroll.to_u16().unwrap(), 0));
		frame.render_widget(widget, area);
	}
}

#[allow(clippy::unused_async)]
async fn title(client: &dyn tg::Client, build: &tg::Build) -> Result<Option<String>> {
	// Get the target.
	let target = build.target(client).await?;

	// Get the package.
	let Some(package) = target.package(client).await? else {
		return Ok(None);
	};

	// Get the metadata.
	let metadata = package.metadata(client).await?;

	// Construct the title.
	let mut title = String::new();
	title.push_str(metadata.name.as_deref().unwrap_or("<unknown>"));
	if let Some(version) = &metadata.version {
		title.push_str(&format!("@{version}"));
	}
	if let Some(name) = target.name(client).await? {
		title.push_str(&format!(":{name}"));
	}

	Ok(Some(title))
}
