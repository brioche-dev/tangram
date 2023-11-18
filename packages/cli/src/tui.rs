use crossterm as ct;
use futures::StreamExt;
use num::ToPrimitive;
use ratatui as tui;
use std::{
	cell::RefCell,
	collections::VecDeque,
	rc::{Rc, Weak},
	sync::{
		atomic::{AtomicBool, AtomicUsize},
		Arc,
	},
};
use tangram_client as tg;
use tangram_error::{Result, WrapErr};
use tangram_lsp::package::Ext;
use tui::{style::Stylize, widgets::Widget};

pub struct Tui {
	#[allow(dead_code)]
	options: Options,
	stop: Arc<AtomicBool>,
	task: Option<tokio::task::JoinHandle<std::io::Result<Terminal>>>,
}

type Backend = tui::backend::CrosstermBackend<std::fs::File>;

type Terminal = tui::Terminal<Backend>;

struct App {
	client: Box<dyn tg::Client>,
	direction: tui::layout::Direction,
	tree: Tree,
	log: Log,
}

struct Tree {
	rect: tui::layout::Rect,
	root: TreeItem,
	scroll: usize,
	selected: TreeItem,
}

#[derive(Clone)]
struct TreeItem {
	inner: Rc<RefCell<TreeItemInner>>,
}

struct TreeItemInner {
	client: Box<dyn tg::Client>,
	build: tg::Build,
	parent: Option<Weak<RefCell<TreeItemInner>>>,
	index: usize,
	selected: bool,
	expanded: bool,
	status: TreeItemStatus,
	title: Option<String>,
	children: Vec<TreeItem>,
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

const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

const SPINNER_FRAMES_PER_UPDATE: usize = 4;

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
		let rect = terminal
			.size()
			.wrap_err("Failed to get the terminal size.")?;

		// Create the stop flag.
		let stop = Arc::new(AtomicBool::new(false));

		// Spawn the task.
		let task = tokio::task::spawn_blocking({
			let client = client.clone_box();
			let build = build.clone();
			let stop = stop.clone();
			move || {
				let mut app = App::new(rect, client.as_ref(), &build);
				while !stop.load(std::sync::atomic::Ordering::SeqCst) {
					if ct::event::poll(std::time::Duration::from_millis(16))? {
						let event = ct::event::read()?;
						if let ct::event::Event::Key(event) = event {
							if options.exit
								&& (event.code == ct::event::KeyCode::Char('q')
									|| (event.code == ct::event::KeyCode::Char('c')
										&& event.modifiers == ct::event::KeyModifiers::CONTROL))
							{
								break;
							}
						}
						app.handle_event(&event);
					}
					app.update();
					terminal.draw(|frame| app.render(frame.size(), frame.buffer_mut()))?;
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
	fn new(rect: tui::layout::Rect, client: &dyn tg::Client, build: &tg::Build) -> Self {
		let client = client.clone_box();
		let direction = tui::layout::Direction::Horizontal;
		let root = TreeItem::new(client.as_ref(), build, None, 0, true, true);
		let tree = Tree::new(rect, root);
		let log = Log::new(client.as_ref(), build);
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
			ct::event::KeyCode::Char('c')
				if event.modifiers == ct::event::KeyModifiers::CONTROL =>
			{
				self.quit();
			},
			ct::event::KeyCode::Char('c') => {
				self.cancel();
			},
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
			ct::event::KeyCode::Char('q') => {
				self.quit();
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
		self.select(true);
	}

	fn up(&mut self) {
		self.select(false);
	}

	fn select(&mut self, down: bool) {
		let expanded_items = self.tree.expanded_items();
		let previous_selected_index = expanded_items
			.iter()
			.position(|item| Rc::ptr_eq(&item.inner, &self.tree.selected.inner))
			.unwrap();
		let new_selected_index = if down {
			(previous_selected_index + 1).min(expanded_items.len() - 1)
		} else {
			previous_selected_index.saturating_sub(1)
		};
		let height = self.tree.rect.height.to_usize().unwrap();
		if new_selected_index < self.tree.scroll {
			self.tree.scroll -= 1;
		} else if new_selected_index >= self.tree.scroll + height {
			self.tree.scroll += 1;
		}
		let new_selected_item = expanded_items[new_selected_index].clone();
		self.tree.selected.inner.borrow_mut().selected = false;
		new_selected_item.inner.borrow_mut().selected = true;
		self.log = Log::new(
			self.client.as_ref(),
			&new_selected_item.inner.borrow().build,
		);
		self.tree.selected = new_selected_item;
	}

	fn expand(&mut self) {
		self.tree.selected.inner.borrow_mut().expanded = true;
	}

	fn collapse(&mut self) {
		if self.tree.selected.inner.borrow().expanded {
			self.tree.selected.inner.borrow_mut().expanded = false;
		} else {
			let parent = self
				.tree
				.selected
				.inner
				.borrow_mut()
				.parent
				.as_ref()
				.map(|parent| TreeItem {
					inner: parent.upgrade().unwrap(),
				});
			if let Some(parent) = parent {
				self.tree.selected.inner.borrow_mut().selected = false;
				self.log = Log::new(self.client.as_ref(), &parent.inner.borrow().build);
				self.tree.selected = parent;
			}
		}
	}

	fn rotate(&mut self) {
		self.direction = match self.direction {
			tui::layout::Direction::Horizontal => tui::layout::Direction::Vertical,
			tui::layout::Direction::Vertical => tui::layout::Direction::Horizontal,
		}
	}

	fn cancel(&mut self) {
		let build = self.tree.selected.inner.borrow().build.clone();
		let client = self.client.clone_box();
		tokio::spawn(async move { build.cancel(client.as_ref()).await.ok() });
	}

	fn quit(&mut self) {
		let build = self.tree.root.inner.borrow().build.clone();
		let client = self.client.clone_box();
		tokio::spawn(async move { build.cancel(client.as_ref()).await.ok() });
	}

	fn render(&mut self, rect: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
		let layout = tui::layout::Layout::default()
			.direction(self.direction)
			.margin(0)
			.constraints([
				tui::layout::Constraint::Percentage(50),
				tui::layout::Constraint::Length(1),
				tui::layout::Constraint::Min(1),
			])
			.split(rect);

		self.tree.render(layout[0], buf);

		let border = match self.direction {
			tui::layout::Direction::Horizontal => tui::widgets::Borders::LEFT,
			tui::layout::Direction::Vertical => tui::widgets::Borders::BOTTOM,
		};
		let block = tui::widgets::Block::default().borders(border);
		block.render(layout[1], buf);

		self.log.render(layout[2], buf);
	}
}

impl Tree {
	fn new(rect: tui::layout::Rect, root: TreeItem) -> Self {
		Self {
			rect,
			root: root.clone(),
			scroll: 0,
			selected: root,
		}
	}

	fn expanded_items(&self) -> Vec<TreeItem> {
		let mut items = Vec::new();
		let mut stack = VecDeque::from(vec![self.root.clone()]);
		while let Some(item) = stack.pop_front() {
			items.push(item.clone());
			if item.inner.borrow().expanded {
				for child in item.inner.borrow().children.iter().rev() {
					stack.push_front(child.clone());
				}
			}
		}
		items
	}

	fn update(&mut self) {
		self.root.update();
	}

	fn render(&mut self, rect: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
		self.rect = rect;
		let layout = tui::layout::Layout::default()
			.direction(tui::layout::Direction::Vertical)
			.constraints(
				(0..rect.height)
					.map(|_| tui::layout::Constraint::Length(1))
					.collect::<Vec<_>>(),
			)
			.split(rect);
		let mut stack = VecDeque::from(vec![self.root.clone()]);
		let mut index = 0;
		while let Some(mut item) = stack.pop_front() {
			if item.inner.borrow().expanded {
				for child in item.inner.borrow().children.iter().rev() {
					stack.push_front(child.clone());
				}
			}
			if index >= self.scroll && index < self.scroll + rect.height.to_usize().unwrap() {
				item.render(layout[index - self.scroll], buf);
			}
			index += 1;
		}
	}
}

impl TreeItem {
	fn new(
		client: &dyn tg::Client,
		build: &tg::Build,
		parent: Option<Weak<RefCell<TreeItemInner>>>,
		index: usize,
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

		let inner = Rc::new(RefCell::new(TreeItemInner {
			client: client.clone_box(),
			build: build.clone(),
			parent,
			index,
			selected,
			expanded,
			status: TreeItemStatus::Building,
			title: None,
			children: Vec::new(),
			status_receiver,
			title_receiver,
			children_receiver,
		}));

		Self { inner }
	}

	fn ancestors(&self) -> Vec<TreeItem> {
		let mut ancestors = Vec::new();
		let mut parent = self.inner.borrow().parent.as_ref().map(|parent| TreeItem {
			inner: parent.upgrade().unwrap(),
		});
		while let Some(parent_) = parent {
			ancestors.push(parent_.clone());
			parent = parent_
				.inner
				.borrow()
				.parent
				.as_ref()
				.map(|parent| TreeItem {
					inner: parent.upgrade().unwrap(),
				});
		}
		ancestors
	}

	fn update(&self) {
		let status = self.inner.borrow_mut().status_receiver.try_recv();
		if let Ok(status) = status {
			self.inner.borrow_mut().status = status;
		}
		let title = self.inner.borrow_mut().title_receiver.try_recv();
		if let Ok(title) = title {
			self.inner.borrow_mut().title = title;
		}
		while let Ok(child) = {
			let child = self.inner.borrow_mut().children_receiver.try_recv();
			child
		} {
			let client = self.inner.borrow().client.clone_box();
			let parent = Some(Rc::downgrade(&self.inner));
			let index = self.inner.borrow().children.len();
			let selected = false;
			let expanded = false;
			let child = TreeItem::new(client.as_ref(), &child, parent, index, selected, expanded);
			self.inner.borrow_mut().children.push(child);
		}
		for child in &self.inner.borrow().children {
			child.update();
		}
	}

	fn render(&mut self, rect: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
		let mut prefix = String::new();
		for item in self.ancestors().iter().rev().skip(1) {
			let parent = item.inner.borrow().parent.clone().unwrap();
			let last =
				item.inner.borrow().index == parent.upgrade().unwrap().borrow().children.len() - 1;
			prefix.push_str(if last { "  " } else { "│ " });
		}
		if let Some(parent) = self.inner.borrow().parent.as_ref() {
			let last =
				self.inner.borrow().index == parent.upgrade().unwrap().borrow().children.len() - 1;
			prefix.push_str(if last { "└─" } else { "├─" });
		}
		let disclosure = if self.inner.borrow().expanded {
			"▼"
		} else {
			"▶"
		};
		let status = match self.inner.borrow().status {
			TreeItemStatus::Building => {
				let state = SPINNER_POSITION.load(std::sync::atomic::Ordering::SeqCst);
				let state = (state / SPINNER_FRAMES_PER_UPDATE) % SPINNER.len();
				SPINNER[state].to_string().blue()
			},
			TreeItemStatus::Failure => "✗".red(),
			TreeItemStatus::Success => "✓".green(),
		};
		let title = self
			.inner
			.borrow()
			.title
			.clone()
			.unwrap_or_else(|| "<unknown>".to_owned());
		let title = tui::text::Line::from(vec![
			prefix.into(),
			disclosure.into(),
			" ".into(),
			status,
			" ".into(),
			title.into(),
		]);
		let style = if self.inner.borrow().selected {
			tui::style::Style::default()
				.bg(tui::style::Color::White)
				.fg(tui::style::Color::Black)
		} else {
			tui::style::Style::default()
		};
		let paragraph = tui::widgets::Paragraph::new(title).style(style);
		paragraph.render(rect, buf);
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

	fn render(&mut self, rect: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
		let text = tui::text::Text::from(self.text.as_str());
		let wrap = tui::widgets::Wrap { trim: false };
		let paragraph = tui::widgets::Paragraph::new(text)
			.wrap(wrap)
			.scroll((self.scroll.to_u16().unwrap(), 0));
		paragraph.render(rect, buf);
	}
}

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
