use self::{
	controller::Controller,
	model::{App, InfoPane},
};
use crossterm as ct;
use ratatui as tui;
use std::{
	os::fd::{AsRawFd, FromRawFd, OwnedFd},
	sync::{atomic::AtomicBool, Arc},
};
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
			Ok(tg::package::Metadata { name, version, .. }) => {
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

pub struct Tui {
	running: Arc<AtomicBool>,
	terminal: Terminal,
	task: Option<tokio::task::JoinHandle<()>>,
}

impl Tui {
	pub fn new(client: &dyn tg::Client, tty: DevTty, root: tg::Build) -> tg::Result<Self> {
		let backend = tui::backend::CrosstermBackend::new(tty);
		let terminal =
			tui::Terminal::new(backend).wrap_err("Failed to create terminal backend.")?;
		let running = Arc::new(AtomicBool::new(true));
		let mut terminal_ = terminal.clone();
		let running_ = running.clone();
		let client = client.clone_box();
		let task = tokio::task::spawn(async move {
			let root_info = info_string(client.as_ref(), &root).await;
			let _ = inner(
				&mut terminal_,
				client.as_ref(),
				root,
				root_info,
				running_.as_ref(),
			)
			.wrap_err("Failed to run TUI.");
			let _ = terminal_.clear();
		});
		let mut tui = Self {
			running,
			terminal,
			task: Some(task),
		};
		tui.setup()?;
		Ok(tui)
	}

	pub fn setup(&mut self) -> tg::Result<()> {
		ct::terminal::enable_raw_mode().wrap_err("Failed to enable terminal raw mode")?;
		ct::execute!(
			self.terminal.backend_mut(),
			ct::event::EnableMouseCapture,
			ct::terminal::EnterAlternateScreen,
		)
		.wrap_err("Failed to setup terminal.")?;
		Ok(())
	}

	pub async fn finish(&mut self) -> tg::Result<()> {
		self.running
			.store(false, std::sync::atomic::Ordering::SeqCst);
		if let Some(task) = self.task.take() {
			let _ = task.await;
		}
		ct::execute!(
			self.terminal.backend_mut(),
			ct::event::DisableMouseCapture,
			ct::terminal::LeaveAlternateScreen
		)
		.wrap_err("Failed to shutdown TUI.")?;
		ct::terminal::disable_raw_mode().wrap_err("Failed to disable terminal raw mode")?;
		Ok(())
	}
}

// /// Run the user interface.
// pub fn ui(client: &dyn tg::Client, tty: DevTty, root: tg::Build, root_info: String) -> Handle {
// 	let running = Arc::new(AtomicBool::new(true));
// 	let running_ = running.clone();
// 	let client = client.clone_box();
// 	let task = tokio::spawn(async move {
// 		tokio::task::spawn_blocking(move || -> tg::Result<()> {
// 			ct::terminal::enable_raw_mode().wrap_err("Failed to enable terminal raw mode")?;
// 			let backend = tui::backend::CrosstermBackend::new(tty);
// 			let mut terminal =
// 				tui::Terminal::new(backend).wrap_err("Failed to create terminal backend.")?;
// 			ct::execute!(
// 				terminal.backend_mut(),
// 				ct::event::EnableMouseCapture,
// 				ct::terminal::EnterAlternateScreen,
// 			)
// 			.wrap_err("Failed to setup TUI")?;
// 			let _ = inner(
// 				&mut terminal,
// 				client.as_ref(),
// 				root,
// 				root_info,
// 				running_.as_ref(),
// 			)
// 			.wrap_err("Failed to run TUI.");
// 			let _ = terminal.clear();

// 			Ok(())
// 		})
// 		.await
// 		.wrap_err("Failed to join task")??;
// 		Ok(())
// 	});

// 	let task = Some(task);
// 	Handle { running, task }
// }

fn inner(
	terminal: &mut Terminal,
	client: &dyn tg::Client,
	root: tg::Build,
	root_info: String,
	running: &AtomicBool,
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
	while running.load(std::sync::atomic::Ordering::SeqCst) {
		// Handle events.
		if ct::event::poll(std::time::Duration::from_millis(16))? {
			let event = ct::event::read()?;
			match event {
				// Special handling for the exit code.
				ct::event::Event::Key(event) if event.code == ct::event::KeyCode::Esc => break,
				ct::event::Event::Key(event) => {
					controller.handle_key_event(event, &mut state);
				},
				ct::event::Event::Mouse(event) => match event.kind {
					ct::event::MouseEventKind::ScrollUp => {
						if let InfoPane::Log(log) = &mut state.info {
							log.scroll_up();
						}
					},
					ct::event::MouseEventKind::ScrollDown => {
						if let InfoPane::Log(log) = &mut state.info {
							log.scroll_down();
						}
					},

					_ => (),
				},
				_ => (),
			}
		}

		// Update the log and build tree states.
		state.update();

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

impl Clone for DevTty {
	fn clone(&self) -> Self {
		let fd = self.fd.try_clone().unwrap();
		Self { fd }
	}
}

impl DevTty {
	pub fn open() -> tg::Result<Self> {
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
