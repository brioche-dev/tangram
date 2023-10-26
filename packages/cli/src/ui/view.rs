use super::{
	controller::Controller,
	model::{App, Build, Log, Status},
	Frame,
};
use itertools::Itertools;
use ratatui as tui;
use std::collections::BTreeMap;

use tui::{
	prelude::*,
	widgets::{Block, Borders, Paragraph, Widget, Wrap},
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
		self.build.view(self.highlighted, frame, layout[0]);
		self.log.view(frame, layout[1]);
	}
}

impl Build {
	fn view(&self, highlighted: usize, frame: &mut Frame<'_>, area: Rect) {
		let page_size = area.height as usize - 1;
		let skip = page_size * (highlighted / page_size);

		let layout = Layout::default()
			.direction(Direction::Vertical)
			.constraints(
				(0..area.height)
					.map(|_| Constraint::Length(1))
					.collect::<Vec<_>>(),
			)
			.split(area);

		let header_layout = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Min(20), Constraint::Max(6)])
			.split(layout[0]);

		for (text, area) in ["Info", "Status"].into_iter().zip(header_layout.iter()) {
			let text = Text::from(text);
			frame.render_widget(Paragraph::new(text), *area);
		}

		self.view_inner(true, skip, highlighted, frame, &layout[1..], 0, "", 0);
	}

	#[allow(clippy::too_many_arguments)]
	fn view_inner(
		&self,
		is_last_child: bool,
		skip: usize,
		highlighted: usize,
		frame: &mut Frame<'_>,
		layout: &[Rect],
		mut offset: usize,
		prefix: &str,
		depth: usize,
	) -> usize {
		if (skip..(skip + layout.len())).contains(&offset) {
			let area = layout[offset - skip];
			let layout = Layout::default()
				.direction(Direction::Horizontal)
				.constraints([Constraint::Min(20), Constraint::Max(6)])
				.split(area);
			let info = &self.info;
			let indicator = if self.children.is_empty() { "" } else { "/" };
			let text = Text::from(format!("{prefix}{info}{indicator}"));
			let style = if highlighted == offset {
				tui::style::Style::default()
					.bg(tui::style::Color::White)
					.fg(tui::style::Color::Black)
			} else {
				tui::style::Style::default()
			};

			frame.render_widget(Paragraph::new(text).style(style), layout[0]);
			frame.render_widget(self.status, layout[1]);
		}

		offset += 1;
		if self.is_expanded {
			for (index, child) in self.children.iter().enumerate() {
				let last_child = index == self.children.len() - 1;
				let end = if last_child { "└─" } else { "├─" };
				let prefix = (0..depth)
					.map(|_| if is_last_child { "  " } else { "│ " })
					.chain(Some(end).into_iter())
					.join("");
				offset = child.view_inner(
					last_child,
					skip,
					highlighted,
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

impl Widget for Status {
	fn render(self, area: tui::prelude::Rect, buf: &mut tui::prelude::Buffer) {
		let char = match self {
			Status::InProgress => {
				const STRING: &str = "⣾⣽⣻⢿⡿⣟⣯⣷";
				let index = unsafe { libc::rand() } % 8;
				STRING.chars().nth(index.try_into().unwrap()).unwrap()
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
				.entry(action)
				.or_insert(Vec::default())
				.push(binding.display());
		}

		let texts = actions
			.into_iter()
			.map(|(action, bindings)| format!("{action}: {}", bindings.join("/")))
			.collect::<Vec<_>>();

		let layout = tui::layout::Layout::default()
			.direction(tui::layout::Direction::Horizontal)
			.constraints(
				(0..texts.len())
					.map(|_| tui::layout::Constraint::Ratio(1, texts.len().try_into().unwrap()))
					.collect::<Vec<_>>(),
			)
			.split(area);

		for (text, area) in texts.into_iter().zip(layout.iter()) {
			let widget = tui::widgets::Paragraph::new(tui::text::Text::from(text));
			frame.render_widget(widget, *area);
		}
	}
}

impl Log {
	fn view(&self, frame: &mut Frame<'_>, mut area: Rect) {
		area.x += 1;
		area.width -= 1;
		let text = Text::from(self.text.as_str());
		let wrap = Wrap { trim: false };
		let paragraph = Paragraph::new(text).wrap(wrap);
		frame.render_widget(paragraph, area);
	}
}
