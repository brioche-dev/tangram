use super::{
	controller::Controller,
	model::{App, Build, BuildResult, InfoPane, Log, Tree},
	Frame,
};
use itertools::Itertools;
use ratatui as tui;
use std::collections::BTreeMap;

use tui::{
	prelude::*,
	widgets::{Block, Borders, Paragraph, Wrap},
};

impl App {
	pub fn view(&mut self, frame: &mut Frame<'_>, area: Rect) {
		let border = match self.direction {
			Direction::Horizontal => Borders::LEFT,
			Direction::Vertical => Borders::BOTTOM,
		};

		let layout = Layout::default()
			.direction(self.direction)
			.margin(0)
			.constraints([
				Constraint::Percentage(50),
				Constraint::Length(1),
				Constraint::Min(1),
			])
			.split(area);

		let block = Block::default().borders(border);
		frame.render_widget(block, layout[1]);
		self.tree.view(frame, layout[0]);
		self.info.view(frame, layout[2]);
	}
}

impl Tree {
	fn view(&mut self, frame: &mut Frame<'_>, area: Rect) {
		let layout = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Length(1), Constraint::Min(1)])
			.split(area);

		// Update the number of skipped children if necessary.
		let height = layout[1].height.try_into().unwrap();
		if self.highlighted < self.skip {
			self.skip = self.highlighted;
		} else if (self.highlighted - self.skip) >= height {
			self.skip = self.highlighted - height + 1;
		}

		let text = Text::from("Builds");
		frame.render_widget(Paragraph::new(text), layout[0]);
		self.root
			.view(self.skip, self.highlighted, frame, layout[1]);
	}
}

impl Build {
	fn view(&self, skip: usize, highlighted: usize, frame: &mut Frame<'_>, area: Rect) {
		let layout = Layout::default()
			.direction(Direction::Vertical)
			.constraints(
				(0..area.height)
					.map(|_| Constraint::Length(1))
					.collect::<Vec<_>>(),
			)
			.split(area);

		self.view_inner(true, skip, highlighted, frame, layout.as_ref(), 0, "", 0);
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
			let info = &self.info;
			let indicator = {
				if self.children.is_empty() {
					" "
				} else if self.is_expanded {
					"▼"
				} else {
					"▶"
				}
			};
			let status = {
				match &self.status {
					Ok(Ok(_)) => "✓",
					Ok(Err(_)) => "❌",
					Err(index) => BuildResult::SPINNER[*index],
				}
			};
			let text = Text::from(format!("{indicator}{status}{prefix}{info}"));
			let style = if highlighted == offset {
				tui::style::Style::default()
					.bg(tui::style::Color::White)
					.fg(tui::style::Color::Black)
			} else {
				tui::style::Style::default()
			};
			frame.render_widget(Paragraph::new(text).style(style), area);
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

impl InfoPane {
	fn view(&self, frame: &mut Frame<'_>, area: Rect) {
		match self {
			Self::Log(log) => log.view(frame, area),
			Self::Result(result) => result.view(frame, area),
		}
	}
}

impl Log {
	fn view(&self, frame: &mut Frame<'_>, area: Rect) {
		let layout = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Length(1), Constraint::Min(1)])
			.split(area);

		let text = Text::from("Log");
		frame.render_widget(Paragraph::new(text), layout[0]);

		let text = Text::from(self.text.as_str());
		let wrap = Wrap { trim: false };
		let widget = Paragraph::new(text).wrap(wrap);
		frame.render_widget(widget, layout[1]);
	}
}

impl BuildResult {
	pub const SPINNER: [&str; 16] = [
		"⣾", "⣾", "⣽", "⣽", "⣻", "⣻", "⢿", "⢿", "⡿", "⡿", "⣟", "⣟", "⣯", "⣯", "⣷", "⣷",
	];
	fn view(&self, frame: &mut Frame<'_>, area: Rect) {
		let layout = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Length(1), Constraint::Min(1)])
			.split(area);

		let text = Text::from("Status");
		frame.render_widget(Paragraph::new(text), layout[0]);

		let text = match &self.value {
			Ok(Ok(value)) => Text::from(format!("✅ {value}")),
			Ok(Err(value)) => Text::from(format!("❌ {value}")),
			Err(state) => Text::from(format!("{} In progress...", Self::SPINNER[*state])),
		};
		let widget = Paragraph::new(text).wrap(Wrap { trim: false });
		frame.render_widget(widget, layout[1]);
	}
}
