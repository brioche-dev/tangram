use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use crossterm as ct;
use ratatui as tui;
use tangram_client as tg;
use tg::WrapErr;

use self::{
	controller::Controller,
	event_stream::{ChildEvent, Event, EventStream},
	state::{App, Build, Status},
};
mod controller;
pub mod event_stream;
mod state;
mod view;

type Backend = tui::backend::CrosstermBackend<DevTty>;
type Frame<'a> = tui::Frame<'a, Backend>;
type Terminal = tui::Terminal<Backend>;

pub async fn ui(client: &dyn tg::Client, root: tg::Build, root_info: String) -> tg::Result<()> {
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

	let _ = inner(&mut terminal, client, root, root_info)
		.await
		.wrap_err("Failed to run TUI.");
	let _ = terminal.clear();

	ct::execute!(
		terminal.backend_mut(),
		ct::event::DisableMouseCapture,
		ct::terminal::LeaveAlternateScreen
	)
	.wrap_err("Failed to shutdown TUI.")?;
	ct::terminal::disable_raw_mode().wrap_err("Failed to disable terminal raw mode")?;
	Ok(())
}

async fn inner(
	terminal: &mut Terminal,
	client: &dyn tg::Client,
	root: tg::Build,
	root_info: String,
) -> std::io::Result<()> {
	// Create the state, event stream, and controller.
	let mut controller = Controller::new();
	let mut state = App::new(client, root.clone(), root_info);
	let events = EventStream::new(std::time::Duration::from_millis(20), client, root.clone());

	// Add the key bindings. Note that these closures take client and events by ref, which means that the Controller instance cannot outlive this function's scope.s
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
		|state| state.scroll_up(),
	);
	controller.add_command(
		"Down",
		[
			(ct::event::KeyCode::Down, ct::event::KeyModifiers::NONE),
			(ct::event::KeyCode::Char('k'), ct::event::KeyModifiers::NONE),
		],
		|state| state.scroll_down(),
	);
	controller.add_command(
		"Open",
		[
			(ct::event::KeyCode::Right, ct::event::KeyModifiers::NONE),
			(ct::event::KeyCode::Char('l'), ct::event::KeyModifiers::NONE),
		],
		|state| state.expand(),
	);
	controller.add_command(
		"Close",
		[
			(ct::event::KeyCode::Left, ct::event::KeyModifiers::NONE),
			(ct::event::KeyCode::Char('h'), ct::event::KeyModifiers::NONE),
		],
		|state| state.collapse(),
	);
	controller.add_command(
		"Rotate",
		[(ct::event::KeyCode::Char('r'), ct::event::KeyModifiers::NONE)],
		|state| state.rotate(),
	);
	let client_ = client.clone_box();
	controller.add_command(
		"Select",
		[(ct::event::KeyCode::Enter, ct::event::KeyModifiers::NONE)],
		move |state| {
			state.select(client_.as_ref());
		},
	);

	// Main loop.
	loop {
		// Yield back to the runtime to avoid starving the thread.
		tokio::task::yield_now().await;

		// Handle events.
		if ct::event::poll(std::time::Duration::from_millis(16))? {
			let event = ct::event::read()?;
			match event {
				// Special handling for the exit code.
				ct::event::Event::Key(event) if event.code == ct::event::KeyCode::Esc => break,
				ct::event::Event::Key(event) => {
					controller.handle_key_event(event, &mut state);
				},
				_ => (),
			}
		}

		// Update the log and build tree states.
		state.log.update();
		state.build.update();

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

			state.view(frame, layout[0]);
			controller.view(frame, layout[1]);
		})?;
	}

	Ok(())
}

pub struct DevTty {
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
