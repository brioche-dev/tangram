use std::{
	collections::{BTreeMap, HashMap},
	os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use crossterm as ct;
use itertools::Itertools;
use ratatui as tui;
use tangram_client as tg;
use tg::WrapErr;

use self::event_stream::{Event, EventStream};
mod controller;
mod event_stream;
mod state;
mod view;

type Backend = tui::backend::CrosstermBackend<DevTty>;
type Frame<'a> = tui::Frame<'a, Backend>;
type Terminal = tui::Terminal<Backend>;

pub fn ui(client: &dyn tg::Client, root: tg::Build) -> tg::Result<()> {
	ct::terminal::enable_raw_mode().wrap_err("Failed to enable terminal raw mode")?;
	let backend = tui::backend::CrosstermBackend::new(DevTty::open()?);
	let mut terminal =
		tui::Terminal::new(backend).wrap_err("Failed to create terminal backend.")?;
	ct::execute!(
		terminal.backend_mut(),
		ct::event::EnableMouseCapture,
		ct::terminal::EnterAlternateScreen,
	)
	.wrap_err("Failed to setup TUI")?;

	let event_stream = EventStream::new(std::time::Duration::from_millis(20), client, root.clone());
	let root = BuildState::with_id(root.id());
	let state = State::new(root);
	do_ui(&mut terminal, state, client, event_stream).wrap_err("Failed to create TUI.")?;

	ct::execute!(
		terminal.backend_mut(),
		ct::event::DisableMouseCapture,
		ct::terminal::LeaveAlternateScreen
	)
	.wrap_err("Failed to shutdown TUI.")?;
	ct::terminal::disable_raw_mode().wrap_err("Failed to disable terminal raw mode")?;
	todo!()
}

fn do_ui(
	terminal: &mut Terminal,
	mut state: State,
	client: &dyn tg::Client,
	event_stream: EventStream,
) -> std::io::Result<()> {
	// Add our keybaard event handlers.
	let mut commands = Commands::new();
	commands.add_command(
		"Exit",
		[(ct::event::KeyCode::Esc, ct::event::KeyModifiers::NONE)],
		|_| {},
	);
	commands.add_command(
		"Up",
		[
			(ct::event::KeyCode::Up, ct::event::KeyModifiers::NONE),
			(ct::event::KeyCode::Char('j'), ct::event::KeyModifiers::NONE),
		],
		|state| state.scroll_up(),
	);
	commands.add_command(
		"Down",
		[
			(ct::event::KeyCode::Down, ct::event::KeyModifiers::NONE),
			(ct::event::KeyCode::Char('k'), ct::event::KeyModifiers::NONE),
		],
		|state| state.scroll_down(),
	);
	commands.add_command(
		"Open",
		[
			(ct::event::KeyCode::Right, ct::event::KeyModifiers::NONE),
			(ct::event::KeyCode::Char('l'), ct::event::KeyModifiers::NONE),
		],
		|state| state.expand(client),
	);
	commands.add_command(
		"Close",
		[
			(ct::event::KeyCode::Left, ct::event::KeyModifiers::NONE),
			(ct::event::KeyCode::Char('h'), ct::event::KeyModifiers::NONE),
		],
		|state| state.collapse(),
	);
	commands.add_command(
		"Rotate",
		[(ct::event::KeyCode::Char('r'), ct::event::KeyModifiers::NONE)],
		|state| state.rotate(),
	);
	commands.add_command(
		"Select",
		[(ct::event::KeyCode::Enter, ct::event::KeyModifiers::NONE)],
		|state| {
			state.select(client, &event_stream);
		},
	);

	loop {
		// Handle events.
		if ct::event::poll(std::time::Duration::from_millis(16))? {
			let event = ct::event::read()?;
			match event {
				// Special handling for the exit code.
				ct::event::Event::Key(event) if event.code == ct::event::KeyCode::Esc => break,
				ct::event::Event::Key(event) => {
					commands.handle_key_event(event, &mut state);
				},
				_ => (),
			}
		}
		for event in event_stream.poll() {
			match event {
				Event::Log(bytes) => {
					let string = match String::from_utf8(bytes) {
						Ok(string) => string,
						Err(e) => e.to_string(),
					};
					state.log_state.push_str(&string);
				},
				Event::Child(child) => {
					if let Some(build) = find_build_by_id_mut(&mut state.builds, child.parent) {
						build.children.push(BuildState::with_id(child.child));
					}
				},
				_ => (), // TODO: handle completions, errors, etc.
			}
		}

		// Render the UI.
		terminal.draw(|frame| {
			let layout = tui::layout::Layout::default()
				.direction(tui::layout::Direction::Vertical)
				.margin(0)
				.constraints([
					tui::layout::Constraint::Min(10),
					tui::layout::Constraint::Max(1),
				])
				.split(frame.size());

			// let text = tui::widgets::Paragraph::new(tui::text::Text::from("Press Esc to exit."));
			// frame.render_widget(text, layout[1]);
			commands.render(frame, layout[1]);
			let area = tui::layout::Layout::default()
				.margin(0)
				.constraints([tui::layout::Constraint::Percentage(100)])
				.split(layout[0]);
			state.render(frame, area[0]);
		})?;
	}
	Ok(())
}

struct State {
	builds: Vec<BuildState>,
	rotation: Rotation,
	selected: usize,
	log_state: String,
}

struct BuildState {
	id: tg::build::Id,
	status: BuildStatus,
	expanded: bool,
	children: Vec<Self>,
}

#[derive(Clone, Copy)]
enum BuildStatus {
	InProgress,
	Successful,
	Error,
}

#[derive(Copy, Clone, Debug)]
enum Rotation {
	Vertical,
	Horizontal,
}

impl State {
	fn new(root: BuildState) -> Self {
		Self {
			builds: vec![root],
			rotation: Rotation::Horizontal,
			selected: 0,
			log_state: "".into(),
		}
	}

	fn select(&mut self, client: &dyn tg::Client, event_stream: &EventStream) {
		let Some(build) = find_build_mut(&mut self.builds, self.selected) else {
			return;
		};
		let build = tg::Build::with_id(build.id.clone());
		self.log_state.clear();
		event_stream.set_log(build)
	}

	fn rotate(&mut self) {
		self.rotation = match self.rotation {
			Rotation::Vertical => Rotation::Horizontal,
			Rotation::Horizontal => Rotation::Vertical,
		};
	}

	fn scroll_up(&mut self) {
		self.selected = self.selected.saturating_sub(1);
	}

	fn scroll_down(&mut self) {
		let len = self
			.builds
			.iter()
			.fold(self.builds.len(), |acc, build| acc + build.len());
		self.selected = self.selected.saturating_add(1).min(len.saturating_sub(1));
	}

	fn expand(&mut self, client: &dyn tg::Client) {
		let which = self.selected;
		let Some(build) = find_build_mut(&mut self.builds, which) else {
			return;
		};
		build.expanded = true;
	}

	fn collapse(&mut self) {
		let which = self.selected;
		let Some(build) = find_build_mut(&mut self.builds, which) else {
			return;
		};
		build.expanded = false;
	}

	fn render(&mut self, frame: &mut Frame<'_>, area: tui::prelude::Rect) {
		let (direction, border) = match self.rotation {
			Rotation::Horizontal => (
				tui::layout::Direction::Horizontal,
				tui::widgets::Borders::LEFT,
			),
			Rotation::Vertical => (tui::layout::Direction::Vertical, tui::widgets::Borders::TOP),
		};

		let layout = tui::layout::Layout::default()
			.direction(direction)
			.margin(0)
			.constraints([
				tui::layout::Constraint::Percentage(50),
				tui::layout::Constraint::Percentage(50),
			])
			.split(area);

		let block = tui::widgets::Block::default().borders(border);
		frame.render_widget(block, layout[1]);

		self.render_build_tree(frame, layout[0]);
		self.render_build_log(frame, layout[1]);
	}

	fn render_build_tree(&mut self, frame: &mut Frame<'_>, area: tui::prelude::Rect) {
		let page_size = area.height as usize - 1;
		let skip = page_size * (self.selected / page_size);

		let vlayout = tui::layout::Layout::default()
			.direction(tui::layout::Direction::Vertical)
			.constraints([
				tui::layout::Constraint::Max(1),
				tui::layout::Constraint::Min(1),
			])
			.split(area);
		let hlayout = tui::layout::Layout::default()
			.direction(tui::layout::Direction::Horizontal)
			.constraints([
				tui::layout::Constraint::Min(12),
				tui::layout::Constraint::Max(10),
				tui::layout::Constraint::Max(10),
			])
			.split(vlayout[0]);

		for (string, area) in ["Target", "Duration", "ID"]
			.into_iter()
			.zip(hlayout.into_iter())
		{
			let widget = tui::widgets::Paragraph::new(tui::text::Text::from(string));
			frame.render_widget(widget, *area);
		}

		let mut offset = 0;
		// let tree_layout = tui::layout::
		for (index, build) in self.builds.iter().enumerate() {
			let is_last_child = index == self.builds.len() - 1;
			offset = build.render(
				frame,
				is_last_child,
				"",
				self.selected,
				skip,
				offset,
				vlayout[1],
				0,
			);
		}
	}

	fn render_build_log(&self, frame: &mut Frame<'_>, area: tui::prelude::Rect) {
		let area = tui::layout::Layout::default()
			.margin(1)
			.constraints([tui::layout::Constraint::Percentage(100)])
			.split(area)[0];
		let text = tui::text::Text::from(&self.log_state as &str);
		let widget = tui::widgets::Paragraph::new(text);
		frame.render_widget(widget, area);
	}
}

impl BuildState {
	fn with_id(id: tg::build::Id) -> Self {
		Self {
			status: BuildStatus::InProgress,
			id: id.clone(),
			children: vec![],
			expanded: false,
		}
	}

	fn len(&self) -> usize {
		self.children
			.iter()
			.fold(self.children.len(), |acc, child| acc + child.len())
	}

	fn render(
		&self,
		frame: &mut Frame<'_>,
		is_last_child: bool,
		tree_str: &str,
		selected: usize,
		skip: usize,
		offset: usize,
		area: tui::prelude::Rect,
		depth: u16,
	) -> usize {
		let count = area.height as usize;
		if (skip..(skip + count)).contains(&offset) {
			let y = (offset - skip) as u16 + area.y;
			let x = area.x + area.x;
			let w = area.width - area.x - 1;
			let h = 1;
			let area = tui::prelude::Rect::new(x, y, w, h);
			let layout = tui::layout::Layout::default()
				.direction(tui::layout::Direction::Horizontal)
				.margin(0)
				.constraints([
					tui::layout::Constraint::Min(12),
					tui::layout::Constraint::Max(2),
					tui::layout::Constraint::Max(8),
					tui::layout::Constraint::Max(8),
				])
				.split(area);

			let id = &self.id;
			let name = &self.id.to_string();
			let tree = format!("{tree_str}{name}");
			let style = if selected == offset {
				tui::style::Style::default()
					.bg(tui::style::Color::White)
					.fg(tui::style::Color::Black)
			} else {
				tui::style::Style::default()
			};

			frame.render_widget(
				tui::widgets::Paragraph::new(tui::text::Text::from(tree)).style(style),
				layout[0],
			);
			frame.render_widget(Status::new(self.status), layout[1]);
			frame.render_widget(
				tui::widgets::Paragraph::new(tui::text::Text::from(id.to_string())).style(style),
				layout[3],
			);
		}

		let mut offset = offset + 1;
		if !self.expanded {
			return offset;
		}

		for (index, child) in self.children.iter().enumerate() {
			let last_child = index == self.children.len() - 1;
			let end = if last_child { "└─" } else { "├─" };
			let tree_str = (0..depth)
				.map(|_| if is_last_child { "  " } else { "│ " })
				.chain(Some(end).into_iter())
				.join("");
			offset = child.render(
				frame,
				last_child,
				&tree_str,
				selected,
				skip,
				offset,
				area,
				depth + 1,
			);
		}

		offset
	}

	fn poll_for_children(&mut self) {}
}

fn find_build<'a>(builds: &'a [BuildState], which: usize) -> Option<&'a BuildState> {
	fn inner<'a>(
		offset: usize,
		which: usize,
		build: &'a BuildState,
	) -> Result<&'a BuildState, usize> {
		if offset == which {
			return Ok(build);
		}
		let mut offset = offset + 1;
		if !build.expanded {
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

fn find_build_mut<'a>(builds: &'a mut [BuildState], which: usize) -> Option<&'a mut BuildState> {
	fn inner<'a>(
		offset: usize,
		which: usize,
		build: &'a mut BuildState,
	) -> Result<&'a mut BuildState, usize> {
		if offset == which {
			return Ok(build);
		}
		let mut offset = offset + 1;
		if !build.expanded {
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

fn find_build_by_id_mut<'a>(
	builds: &'a mut [BuildState],
	id: tg::build::Id,
) -> Option<&'a mut BuildState> {
	fn inner<'a>(id: tg::build::Id, build: &'a mut BuildState) -> Option<&'a mut BuildState> {
		if build.id == id {
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

struct DevTty {
	fd: std::os::fd::OwnedFd,
}

impl DevTty {
	fn open() -> tg::Result<Self> {
		unsafe {
			let fd = libc::open(b"/dev/tty\0".as_ptr().cast(), libc::O_RDWR);
			if fd < 0 {
				Err(std::io::Error::last_os_error()).wrap_err("Failed to open /dev/tty")?;
			}
			let fd = OwnedFd::from_raw_fd(fd);
			Ok(Self { fd })
		}
	}
}

impl std::io::Write for DevTty {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		unsafe {
			let fd = self.fd.as_raw_fd();
			let ret = libc::write(fd, buf.as_ptr().cast(), buf.len());
			if ret < 0 {
				return Err(std::io::Error::last_os_error());
			} else {
				Ok(ret as usize)
			}
		}
	}
	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

struct Commands {
	actions: BTreeMap<String, Box<dyn Fn(&mut State)>>,
	bindings: HashMap<KeyBinding, String>,
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
struct KeyBinding(ct::event::KeyCode, ct::event::KeyModifiers);

impl Commands {
	fn new() -> Self {
		Self {
			actions: BTreeMap::default(),
			bindings: HashMap::default(),
		}
	}

	fn add_command(
		&mut self,
		name: &str,
		bindings: impl IntoIterator<Item = (ct::event::KeyCode, ct::event::KeyModifiers)>,
		action: impl Fn(&mut State) + 'static,
	) {
		let action: Box<dyn Fn(&mut State)> = Box::new(action);
		self.actions.insert(name.into(), action);
		for binding in bindings {
			self.bindings
				.insert(KeyBinding(binding.0, binding.1), name.into());
		}
	}

	fn render(&self, frame: &mut Frame<'_>, area: tui::prelude::Rect) {
		let mut actions = BTreeMap::default();

		for (binding, action) in &self.bindings {
			actions
				.entry(action.to_owned())
				.or_insert(Vec::default())
				.push(display_binding(binding))
		}

		let texts = actions
			.into_iter()
			.map(|(action, bindings)| format!("{action}: {}", bindings.join("/")))
			.collect::<Vec<_>>();

		let layout = tui::layout::Layout::default()
			.direction(tui::layout::Direction::Horizontal)
			.constraints(
				(0..texts.len())
					.map(|_| tui::layout::Constraint::Ratio(1, texts.len() as u32))
					.collect::<Vec<_>>(),
			)
			.split(area);

		for (text, area) in texts.into_iter().zip(layout.into_iter()) {
			let widget = tui::widgets::Paragraph::new(tui::text::Text::from(text));
			frame.render_widget(widget, *area);
		}
	}

	fn handle_key_event(&self, event: ct::event::KeyEvent, state: &mut State) {
		let binding = KeyBinding(event.code, event.modifiers);
		if let Some(name) = self.bindings.get(&binding) {
			let action = self.actions.get(name).unwrap();
			action(state);
		}
	}
}

fn display_binding(binding: &KeyBinding) -> String {
	let mut buf = String::new();
	for modifier in binding.1 {
		match modifier {
			ct::event::KeyModifiers::SHIFT => buf.push('⇧'),
			ct::event::KeyModifiers::CONTROL => buf.push('⌃'),
			ct::event::KeyModifiers::ALT => buf.push_str("ALT"),
			ct::event::KeyModifiers::SUPER => buf.push('⌘'),
			ct::event::KeyModifiers::HYPER => buf.push('⌥'),
			ct::event::KeyModifiers::META => buf.push('⌥'),
			_ => continue,
		}
		buf.push('+')
	}

	match binding.0 {
		ct::event::KeyCode::Backspace => buf.push('⌫'),
		ct::event::KeyCode::Enter => buf.push('⏎'),
		ct::event::KeyCode::Left => buf.push('←'),
		ct::event::KeyCode::Right => buf.push('→'),
		ct::event::KeyCode::Up => buf.push('↑'),
		ct::event::KeyCode::Down => buf.push('↓'),
		ct::event::KeyCode::Home => buf.push('⇱'),
		ct::event::KeyCode::End => buf.push_str("⇲"),
		ct::event::KeyCode::PageUp => buf.push('⇞'),
		ct::event::KeyCode::PageDown => buf.push('⇟'),
		ct::event::KeyCode::Tab => buf.push('⇥'),
		ct::event::KeyCode::BackTab => buf.push('⭰'),
		ct::event::KeyCode::Delete => buf.push('⌦'),
		ct::event::KeyCode::F(num) => {
			buf.push('F');
			buf.push_str(&num.to_string());
		},
		ct::event::KeyCode::Char(char) => buf.extend(char.to_uppercase()),
		ct::event::KeyCode::Null => buf.push('\0'),
		ct::event::KeyCode::Esc => buf.push('⎋'),
		ct::event::KeyCode::CapsLock => buf.push('⇪'),
		key => buf.push_str(&format!("{key:?}")),
	}
	buf
}

struct Status(BuildStatus);
impl Status {
	fn new(status: BuildStatus) -> Self {
		Self(status)
	}
}

impl tui::widgets::Widget for Status {
	fn render(self, area: tui::prelude::Rect, buf: &mut tui::prelude::Buffer) {
		let char = match self.0 {
			BuildStatus::InProgress => {
				const STRING: &str = "⣾⣽⣻⢿⡿⣟⣯⣷";
				let index = unsafe { libc::rand() } as usize % 8;
				STRING.chars().nth(index).unwrap()
			},
			BuildStatus::Successful => '✅',
			BuildStatus::Error => '❌',
		};
		let string = format!("{char}");
		buf.set_string(area.x, area.y, string, tui::style::Style::default());
	}
}
