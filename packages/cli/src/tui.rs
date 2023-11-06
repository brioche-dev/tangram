use crossterm as ct;
use futures::StreamExt;
use itertools::Itertools;
use num::ToPrimitive;
use ratatui as tui;
use std::sync::{
	atomic::{AtomicBool, AtomicUsize},
	Arc,
};
use tangram_client as tg;
use tg::{Result, WrapErr};

type Backend = tui::backend::CrosstermBackend<std::fs::File>;

type Terminal = tui::Terminal<Backend>;

type Frame<'a> = tui::Frame<'a, Backend>;

pub struct Tui {
	finish: Arc<AtomicBool>,
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

impl Tui {
	pub async fn start(client: &dyn tg::Client, build: &tg::Build) -> Result<Self> {
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

		// Create the finish flag.
		let finish = Arc::new(AtomicBool::new(false));

		// Spawn the task.
		let task = tokio::task::spawn_blocking({
			let client = client.clone_box();
			let build = build.clone();
			let finish = finish.clone();
			move || {
				let mut app = App::new(client.as_ref(), &build);
				while !finish.load(std::sync::atomic::Ordering::SeqCst) {
					if ct::event::poll(std::time::Duration::from_millis(16))? {
						app.handle_event(&ct::event::read()?);
					}
					app.update();
					terminal.draw(|frame| app.render(frame, frame.size()))?;
				}
				Ok(terminal)
			}
		});

		Ok(Self {
			finish,
			task: Some(task),
		})
	}

	pub async fn finish(mut self) -> Result<()> {
		// Set the finish flag.
		self.finish.store(true, std::sync::atomic::Ordering::SeqCst);

		// Wait for the task to finish.
		let mut terminal = self
			.task
			.take()
			.unwrap()
			.await
			.wrap_err("Failed to join the task.")?
			.wrap_err("The task did not succeed.")?;

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
		let tree = Tree::new(TreeItem::new(client.as_ref(), root));
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
				.min(self.tree.root.size()),
		);
	}

	fn up(&mut self) {
		self.select(self.tree.selected.saturating_sub(1));
	}

	fn select(&mut self, index: usize) {
		self.tree.selected = index;
		let selected_item = self.selected_item();
		self.log = Log::new(self.client.as_ref(), &selected_item.build);
	}

	fn expand(&mut self) {
		let item = self.selected_item_mut();
		item.expanded = true;
	}

	fn collapse(&mut self) {
		let item = self.selected_item_mut();
		item.expanded = false;
	}

	fn rotate(&mut self) {
		self.direction = match self.direction {
			tui::layout::Direction::Horizontal => tui::layout::Direction::Vertical,
			tui::layout::Direction::Vertical => tui::layout::Direction::Horizontal,
		}
	}

	fn selected_item(&self) -> &'_ TreeItem {
		fn inner(item: &'_ TreeItem, offset: usize, needle: usize) -> Result<&'_ TreeItem, usize> {
			if offset == needle {
				return Ok(item);
			}
			let mut offset = offset + 1;
			if !item.expanded {
				return Err(offset);
			}
			for child in &item.children {
				match inner(child, offset, needle) {
					Ok(found) => return Ok(found),
					Err(o) => offset = o,
				}
			}
			Err(offset)
		}
		inner(&self.tree.root, 0, self.tree.selected).unwrap()
	}

	fn selected_item_mut(&mut self) -> &'_ mut TreeItem {
		fn inner(
			item: &'_ mut TreeItem,
			offset: usize,
			needle: usize,
		) -> Result<&'_ mut TreeItem, usize> {
			if offset == needle {
				return Ok(item);
			}
			let mut offset = offset + 1;
			if !item.expanded {
				return Err(offset);
			}
			for child in &mut item.children {
				match inner(child, offset, needle) {
					Ok(found) => return Ok(found),
					Err(o) => offset = o,
				}
			}
			Err(offset)
		}
		inner(&mut self.tree.root, 0, self.tree.selected).unwrap()
	}

	fn render(&mut self, frame: &mut Frame, area: tui::layout::Rect) {
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

	fn update(&mut self) {
		self.root.update();
	}

	fn render(&mut self, frame: &mut Frame, area: tui::layout::Rect) {
		// Update the number of skipped children if necessary.
		let height = area.height.to_usize().unwrap();
		if self.selected < self.scroll {
			self.scroll = self.selected;
		} else if (self.selected - self.scroll) >= height {
			self.scroll = self.selected - height + 1;
		}

		self.root.render(self.scroll, self.selected, frame, area);
	}
}

impl TreeItem {
	fn new(client: &dyn tg::Client, build: &tg::Build) -> Self {
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
				let title = title(client.as_ref(), &build).await;
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
			expanded: false,
			status: TreeItemStatus::Building,
			title: None,
			children: Vec::new(),
			status_receiver,
			title_receiver,
			children_receiver,
		}
	}

	fn size(&self) -> usize {
		self.children
			.iter()
			.fold(self.children.len(), |len, child| len + child.size())
	}

	fn update(&mut self) {
		if let Ok(status) = self.status_receiver.try_recv() {
			self.status = status;
		}
		if let Ok(title) = self.title_receiver.try_recv() {
			self.title = title;
		}
		while let Ok(child) = self.children_receiver.try_recv() {
			let child = TreeItem::new(self.client.as_ref(), &child);
			self.children.push(child);
		}
		for child in &mut self.children {
			child.update();
		}
	}

	fn render(
		&mut self,
		scroll: usize,
		selected: usize,
		frame: &mut Frame,
		area: tui::layout::Rect,
	) {
		let layout = tui::layout::Layout::default()
			.direction(tui::layout::Direction::Vertical)
			.constraints(
				(0..area.height)
					.map(|_| tui::layout::Constraint::Length(1))
					.collect::<Vec<_>>(),
			)
			.split(area);
		self.render_inner(true, scroll, selected, frame, layout.as_ref(), 0, "", 0);
	}

	#[allow(clippy::too_many_arguments)]
	fn render_inner(
		&self,
		is_last_child: bool,
		scroll: usize,
		selected: usize,
		frame: &mut Frame,
		layout: &[tui::layout::Rect],
		mut offset: usize,
		prefix: &str,
		depth: usize,
	) -> usize {
		if (scroll..(scroll + layout.len())).contains(&offset) {
			let area = layout[offset - scroll];
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
			let style = if selected == offset {
				tui::style::Style::default()
					.bg(tui::style::Color::White)
					.fg(tui::style::Color::Black)
			} else {
				tui::style::Style::default()
			};
			frame.render_widget(tui::widgets::Paragraph::new(title).style(style), area);
		}
		offset += 1;
		if self.expanded {
			for (index, child) in self.children.iter().enumerate() {
				let last_child = index == self.children.len() - 1;
				let end = if last_child { "└─" } else { "├─" };
				let prefix = (0..depth)
					.map(|_| if is_last_child { "  " } else { "│ " })
					.chain(Some(end).into_iter())
					.join("");
				offset = child.render_inner(
					last_child,
					scroll,
					selected,
					frame,
					layout,
					offset,
					&prefix,
					depth + 1,
				);
			}
		}
		offset
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

	fn render(&mut self, frame: &mut Frame, area: tui::layout::Rect) {
		let text = tui::text::Text::from(self.text.as_str());
		let wrap = tui::widgets::Wrap { trim: false };
		let widget = tui::widgets::Paragraph::new(text)
			.wrap(wrap)
			.scroll((self.scroll.to_u16().unwrap(), 0));
		frame.render_widget(widget, area);
	}
}

#[allow(clippy::unused_async)]
async fn title(_client: &dyn tg::Client, _build: &tg::Build) -> Option<String> {
	None
}
