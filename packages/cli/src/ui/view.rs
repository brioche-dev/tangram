use std::collections::BTreeMap;

use super::{controller::Controller, state::*, Frame};
use itertools::Itertools;
use ratatui as tui;
use tui::{
	prelude::*,
	widgets::{Block, Borders, Paragraph, Widget},
};

impl App {
	pub fn view(&self, frame: &mut Frame<'_>, area: Rect) {
		let border = match self.direction {
			Direction::Horizontal => Borders::LEFT,
			Direction::Vertical => Borders::BOTTOM,
		};

		let layout = Layout::default()
			.direction(self.direction)
			.margin(0)
			.constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
			.split(area);

		let block = Block::default().borders(border);
		frame.render_widget(block, layout[1]);
		self.view_builds(frame, layout[0]);
		self.view_log(frame, layout[1]);
	}

	fn view_builds(&self, frame: &mut Frame<'_>, area: tui::prelude::Rect) {
		let page_size = area.height as usize - 1;
		let skip = page_size * (self.highlighted / page_size);

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

		for (string, area) in ["ID", "Status", "id"].into_iter().zip(hlayout.into_iter()) {
			let widget = tui::widgets::Paragraph::new(tui::text::Text::from(string));
			frame.render_widget(widget, *area);
		}

		let mut offset = 0;
		// let tree_layout = tui::layout::
		for (index, build) in self.builds.iter().enumerate() {
			let is_last_child = index == self.builds.len() - 1;
			offset = build.view(
				frame,
				is_last_child,
				"",
				self.highlighted,
				skip,
				offset,
				vlayout[1],
				0,
			);
		}
	}

	fn view_log(&self, frame: &mut Frame<'_>, area: Rect) {
		let build = self.selected_build();
		let log = build.log.as_deref().unwrap_or("");
		let text = Text::from(log);
		let widget = Paragraph::new(text);
		frame.render_widget(widget, area);
	}
}

impl Build {
	fn view(
		&self,
		frame: &mut Frame<'_>,
		is_last_child: bool,
		tree_str: &str,
		selected: usize,
		skip: usize,
		offset: usize,
		area: Rect,
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

			let id = &self.build.id();
			let name = id.to_string();
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
			frame.render_widget(self.status, layout[1]);
			frame.render_widget(
				tui::widgets::Paragraph::new(tui::text::Text::from(id.to_string())).style(style),
				layout[3],
			);
		}

		let mut offset = offset + 1;
		if !self.is_expanded {
			return offset;
		}

		for (index, child) in self.children.iter().enumerate() {
			let last_child = index == self.children.len() - 1;
			let end = if last_child { "└─" } else { "├─" };
			let tree_str = (0..depth)
				.map(|_| if is_last_child { "  " } else { "│ " })
				.chain(Some(end).into_iter())
				.join("");
			offset = child.view(
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

impl Widget for Status {
	fn render(self, area: tui::prelude::Rect, buf: &mut tui::prelude::Buffer) {
		let char = match self {
			Status::InProgress => {
				const STRING: &str = "⣾⣽⣻⢿⡿⣟⣯⣷";
				let index = unsafe { libc::rand() } as usize % 8;
				STRING.chars().nth(index).unwrap()
			},
			Status::Successful => '✅',
			Status::Error => '❌',
		};
		let string = format!("{char}");
		buf.set_string(area.x, area.y, string, tui::style::Style::default());
	}
}

impl Controller {
	pub fn view(&self, frame: &mut Frame<'_>, area: Rect) {
		let mut actions = BTreeMap::default();

		for (binding, action) in &self.bindings {
			actions
				.entry(action.to_owned())
				.or_insert(Vec::default())
				.push(binding.display())
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
}
