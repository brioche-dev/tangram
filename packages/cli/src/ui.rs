use crossterm as ct;
use itertools::Itertools;
use ratatui as tui;
use tangram_client as tg;

type Backend = tui::backend::CrosstermBackend<std::io::Stdout>;
type Frame<'a> = tui::Frame<'a, Backend>;
type Terminal = tui::Terminal<Backend>;

pub fn ui() -> tg::Result<()> {
	ct::terminal::enable_raw_mode()?;
	let backend = tui::backend::CrosstermBackend::new(std::io::stdout());
	let mut terminal = tui::Terminal::new(backend)?;
	ct::execute!(
		terminal.backend_mut(),
		ct::event::EnableMouseCapture,
		ct::terminal::EnterAlternateScreen,
	)?;

	do_ui(&mut terminal)?;

	ct::execute!(
		terminal.backend_mut(),
		ct::event::DisableMouseCapture,
		ct::terminal::LeaveAlternateScreen
	)?;
	ct::terminal::disable_raw_mode()?;
	todo!()
}

fn do_ui(terminal: &mut Terminal) -> std::io::Result<()> {
	let mut state = dummy_state();

	loop {
		if ct::event::poll(std::time::Duration::from_millis(16))? {
			let event = ct::event::read()?;
			match event {
				ct::event::Event::Key(event) => match event.code {
					ct::event::KeyCode::Esc => break,
					ct::event::KeyCode::Up => state.scroll_up(),
					ct::event::KeyCode::Down => state.scroll_down(),
					ct::event::KeyCode::Left => state.collapse(),
					ct::event::KeyCode::Right => state.expand(),
					ct::event::KeyCode::Char(c) => match c {
						'j' => state.scroll_up(),
						'k' => state.scroll_down(),
						'h' => state.collapse(),
						'l' => state.expand(),
						_ => (),
					},
					_ => (),
				},
				_ => (),
			}
		}

		terminal.draw(|frame| {
			let layout = tui::layout::Layout::default()
				.direction(tui::layout::Direction::Vertical)
				.margin(1)
				.constraints([
					tui::layout::Constraint::Percentage(90),
					tui::layout::Constraint::Percentage(10),
				])
				.split(frame.size());
			// eprintln!("drawing");
			let block = tui::widgets::Block::default()
				.title("Tangram")
				.borders(tui::widgets::Borders::ALL);

			let text = tui::widgets::Paragraph::new(tui::text::Text::from("Press Esc to exit."));
			frame.render_widget(block, layout[0]);
			frame.render_widget(text, layout[1]);

			let area = tui::layout::Layout::default()
				.margin(1)
				.constraints([tui::layout::Constraint::Percentage(100)])
				.split(layout[0]);
			state.render(frame, area[0]);
		})?;
	}
	Ok(())
}

struct State {
	builds: Vec<BuildState>,
	selected: usize,
}

struct BuildState {
	id: String,
	name: String,
	time: String,
	children: Vec<Self>,
}

impl State {
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

	fn expand(&mut self) {
		let which = self.selected;
		let Some(build) = find_build_mut(&mut self.builds, which) else {
			return;
		};
		build.children = get_children(&build.name);
	}

	fn collapse(&mut self) {
		let which = self.selected;
		let Some(build) = find_build_mut(&mut self.builds, which) else {
			return;
		};
		build.children.clear();
	}

	fn render(&mut self, frame: &mut Frame<'_>, area: tui::prelude::Rect) {
		let skip = self.selected / area.height as usize;
		let mut offset = 0;
		for (index, build) in self.builds.iter().enumerate() {
			let is_last_child = index == self.builds.len() - 1;
			offset = build.render(
				frame,
				is_last_child,
				"",
				self.selected,
				skip,
				offset,
				area,
				0,
			);
		}
	}
}

impl BuildState {
	fn with_name(name: &str) -> Self {
		Self {
			id: "<ID>".into(),
			name: name.into(),
			time: "123.45".into(),
			children: vec![],
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
			let id = &self.id;
			let name = &self.name;
			let time = &self.time;
			let text = tui::text::Text::from(format!("{id} {time} {tree_str}{name}"));
			let style = if selected == offset {
				tui::style::Style::default()
					.bg(tui::style::Color::White)
					.fg(tui::style::Color::Black)
			} else {
				tui::style::Style::default()
			};
			let widget = tui::widgets::Paragraph::new(text).style(style);

			let y = (offset - skip) as u16 + area.y;
			let x = area.x + area.x;
			let w = area.width - area.x - 1;
			let h = 1;

			let area = tui::prelude::Rect::new(x, y, w, h);
			frame.render_widget(widget, area);
		}

		let mut offset = offset + 1;
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
}

fn dummy_state() -> State {
	State {
		selected: 0,
		builds: vec![
			BuildState::with_name("target_1"),
			BuildState::with_name("target_3"),
			BuildState::with_name("target_10"),
		],
	}
}

// R: Rotate (top bottom, left right)
// J/K: Up/Down
// H/L: Open/Close
// One dividing line between tree and output
//

/*

	<ID> <NAME>
	<ID> ├─<NAME>
	<ID> │ ├─<NAME>
	<ID> │ └─<NAME>
	<ID> └─<NAME>

	// render line
	<id> <space> <tree string> <name>
*/

fn get_children(name: &str) -> Vec<BuildState> {
	match name {
		"target_1" => vec![BuildState::with_name("target_2")],
		"target_3" => vec![
			BuildState::with_name("target_4"),
			BuildState::with_name("target_5"),
			BuildState::with_name("target_7"),
		],
		"target_7" => vec![
			BuildState::with_name("target_8"),
			BuildState::with_name("target_9"),
		],
		"target_10" => vec![
			BuildState::with_name("target_11"),
			BuildState::with_name("target_12"),
			BuildState::with_name("target_16"),
			BuildState::with_name("target_17"),
		],
		"target_12" => vec![
			BuildState::with_name("target_13"),
			BuildState::with_name("target_14"),
			BuildState::with_name("target_15"),
		],
		_ => vec![],
	}
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

// fn find_build<'a>(builds: &'a [BuildState], which: usize) -> Option<&'a BuildState> {
// 	fn inner<'a>(
// 		offset: usize,
// 		which: usize,
// 		build: &'a BuildState,
// 	) -> Result<&'a BuildState, usize> {
// 		if offset == which {
// 			return Ok(build);
// 		}
// 		let mut offset = offset + 1;
// 		for child in &build.children {
// 			match inner(offset, which, child) {
// 				Ok(found) => return Ok(found),
// 				Err(o) => offset = offset,
// 			}
// 		}
// 		return Err(offset);
// 	}

// 	let mut offset = 0;
// 	for build in builds {
// 		match inner(offset, which, build) {
// 			Ok(found) => return Some(found),
// 			Err(o) => offset = o,
// 		}
// 	}
// 	None
// }
