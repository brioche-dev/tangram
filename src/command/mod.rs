/// A command.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
	/// The system to run the command on.
	host: System,

	/// The command's executable.
	executable: Template,

	/// The command's environment variables.
	#[serde(default)]
	env: BTreeMap<String, Template>,

	/// The command's command line arguments.
	#[serde(default)]
	args: Vec<Template>,

	/// If this flag is set, then the command will have access to the network.
	#[serde(default)]
	network: bool,
}

pub enum Input {
	Stdin(Vec<u8>),
	Window((u64, u64)),
	Signal(i32),
}

pub enum Event {
	Stdout(Vec<u8>),
	Stderr(Vec<u8>),
	Exit(Exit),
}

enum Exit {
	Code(i32),
	Signal(i32),
}
