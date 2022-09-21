use anyhow::Result;
use clap::{CommandFactory, Parser};
use deno_core::anyhow::Context;
use indoc::formatdoc;
use std::fmt::Write;

#[derive(Parser)]
pub struct Args {
	#[clap(arg_enum)]
	shell: Shell,
}

#[derive(Clone, Copy, clap::ArgEnum)]
enum Shell {
	Bash,
	Zsh,
}

#[allow(clippy::unused_async)]
pub async fn run(args: Args) -> Result<()> {
	let mut shellhook = String::new();

	// Generate the completion.
	let mut completion_bytes = Vec::new();
	let clap_shell = match args.shell {
		Shell::Bash => clap_complete::Shell::Bash,
		Shell::Zsh => clap_complete::Shell::Zsh,
	};
	let mut command = crate::Args::command();
	let name = "tg";
	clap_complete::generate(clap_shell, &mut command, name, &mut completion_bytes);
	let completion = String::from_utf8(completion_bytes)
		.context("The generated completion script was not valid UTF-8.")?;

	match args.shell {
		Shell::Bash => {},
		Shell::Zsh => {
			let completion = completion.replace(&format!("#compdef {name}"), "");
			let completion = completion.replace(&format!(r#"_{name} "$@""#), "");
			let completion = completion.trim();
			writeln!(&mut shellhook, "{}", completion).unwrap();
			writeln!(&mut shellhook).unwrap();
			writeln!(
				&mut shellhook,
				"{}",
				formatdoc!(
					r#"
						autoload -U compinit
						compinit
						compdef _tg tg
					"#
				)
			)
			.unwrap();
		},
	}

	match args.shell {
		Shell::Bash => {
			writeln!(
				&mut shellhook,
				"{}",
				formatdoc!(
					r#"
					"#
				)
			)
			.unwrap();
		},
		Shell::Zsh => {},
	};

	// Print the shellhook to stdout.
	let shellhook = shellhook.trim();
	println!("{shellhook}");

	Ok(())
}
