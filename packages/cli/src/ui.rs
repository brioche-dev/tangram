use self::{controller::Controller, model::App};
use crossterm as ct;
use ratatui as tui;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use tangram_client as tg;
use tangram_package::PackageExt;
use tg::WrapErr;

mod controller;
mod model;
mod view;

type Backend = tui::backend::CrosstermBackend<DevTty>;
type Frame<'a> = tui::Frame<'a, Backend>;
type Terminal = tui::Terminal<Backend>;

// Helper function to display an informatic string about a build.
pub async fn info_string(client: &dyn tg::Client, build: &tg::Build) -> String {
	let target = match build.target(client).await {
		Ok(target) => target,
		Err(e) => return format!("Error: {e}"),
	};

	let name = match target.name(client).await {
		Ok(Some(name)) => name.clone(),
		Ok(None) => "<unknown>".into(),
		Err(e) => format!("Error: {e}"),
	};

	let package = match target.package(client).await {
		Ok(Some(package)) => match package.metadata(client).await {
			Ok(tg::package::Metadata { name, version }) => {
				let name = name.as_deref().unwrap_or("<unknown>");
				let version = version.as_deref().unwrap_or("<unknown>");
				format!("{name}@{version}")
			},
			Err(e) => format!("Error: {e}"),
		},
		Ok(None) => "<unknown>".into(),
		Err(e) => format!("Error: {e}"),
	};

	format!("{package}: {name}")
}

/// Run the user interface.
pub fn ui(client: &dyn tg::Client, root: tg::Build, root_info: String) -> tg::Result<()> {
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

	let _ = inner(&mut terminal, client, root, root_info).wrap_err("Failed to run TUI.");
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

fn inner(
	terminal: &mut Terminal,
	client: &dyn tg::Client,
	root: tg::Build,
	root_info: String,
) -> std::io::Result<()> {
	// Create the state, event stream, and controller.
	let mut controller = Controller::new();
	let mut state = App::new(client, root, root_info);

	// Add the key bindings. Note that these closures take client and events by ref, which means that the Controller instance cannot outlive this function's scope.s
	// The "Select" commands is special because it requires a `Client`.
	let client_ = client.clone_box();
	controller.add_command(
		"Select",
		[(ct::event::KeyCode::Enter, ct::event::KeyModifiers::NONE)],
		move |state| {
			state.select(client_.as_ref());
		},
	);
	let client_ = client.clone_box();
	controller.add_command(
		"Tab Info",
		[(ct::event::KeyCode::Tab, ct::event::KeyModifiers::NONE)],
		move |state| {
			state.tab_info(client_.as_ref());
		},
	);

	// Main loop.
	loop {
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
		state.info.update();
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
				Err(std::io::Error::last_os_error())
			} else {
				Ok(ret.try_into().unwrap())
			}
		}
	}
	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}
