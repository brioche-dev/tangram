use super::model::App;
use crossterm as ct;
use std::collections::{BTreeMap, HashMap};

type Action = Box<dyn Fn(&mut App) + Send + 'static>;
pub struct Controller {
	pub actions: BTreeMap<String, Action>,
	pub bindings: HashMap<KeyBinding, String>,
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
pub struct KeyBinding(ct::event::KeyCode, ct::event::KeyModifiers);

impl Controller {
	pub fn new() -> Self {
		let mut controller = Self {
			actions: BTreeMap::default(),
			bindings: HashMap::default(),
		};
		controller.add_command(
			"Exit",
			[(ct::event::KeyCode::Esc, ct::event::KeyModifiers::NONE)],
			|_| {},
		);
		controller.add_command(
			"Up",
			[
				(ct::event::KeyCode::Up, ct::event::KeyModifiers::NONE),
				(ct::event::KeyCode::Char('j'), ct::event::KeyModifiers::NONE),
			],
			App::scroll_up,
		);
		controller.add_command(
			"Down",
			[
				(ct::event::KeyCode::Down, ct::event::KeyModifiers::NONE),
				(ct::event::KeyCode::Char('k'), ct::event::KeyModifiers::NONE),
			],
			App::scroll_down,
		);
		controller.add_command(
			"Open",
			[
				(ct::event::KeyCode::Right, ct::event::KeyModifiers::NONE),
				(ct::event::KeyCode::Char('l'), ct::event::KeyModifiers::NONE),
			],
			App::expand,
		);
		controller.add_command(
			"Close",
			[
				(ct::event::KeyCode::Left, ct::event::KeyModifiers::NONE),
				(ct::event::KeyCode::Char('h'), ct::event::KeyModifiers::NONE),
			],
			App::collapse,
		);
		controller.add_command(
			"Rotate",
			[(ct::event::KeyCode::Char('r'), ct::event::KeyModifiers::NONE)],
			App::rotate,
		);
		controller
	}

	pub fn add_command(
		&mut self,
		name: &str,
		bindings: impl IntoIterator<Item = (ct::event::KeyCode, ct::event::KeyModifiers)>,
		action: impl Fn(&mut App) + Send + 'static,
	) {
		let action = Box::new(action);
		self.actions.insert(name.into(), action);
		for binding in bindings {
			self.bindings
				.insert(KeyBinding(binding.0, binding.1), name.into());
		}
	}

	pub fn handle_key_event(&self, event: ct::event::KeyEvent, state: &mut App) {
		let binding = KeyBinding(event.code, event.modifiers);
		if let Some(name) = self.bindings.get(&binding) {
			let action = self.actions.get(name).unwrap();
			action(state);
		}
	}
}

impl KeyBinding {
	pub fn display(&self) -> String {
		let mut buf = String::new();
		for modifier in self.1 {
			match modifier {
				ct::event::KeyModifiers::SHIFT => buf.push('⇧'),
				ct::event::KeyModifiers::CONTROL => buf.push('⌃'),
				ct::event::KeyModifiers::ALT => buf.push_str("ALT"),
				ct::event::KeyModifiers::SUPER => buf.push('⌘'),
				ct::event::KeyModifiers::HYPER | ct::event::KeyModifiers::META => buf.push('⌥'),
				_ => continue,
			}
			buf.push('+');
		}

		match self.0 {
			ct::event::KeyCode::Backspace => buf.push('⌫'),
			ct::event::KeyCode::Enter => buf.push('⏎'),
			ct::event::KeyCode::Left => buf.push('←'),
			ct::event::KeyCode::Right => buf.push('→'),
			ct::event::KeyCode::Up => buf.push('↑'),
			ct::event::KeyCode::Down => buf.push('↓'),
			ct::event::KeyCode::Home => buf.push('⇱'),
			ct::event::KeyCode::End => buf.push('⇲'),
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
}
