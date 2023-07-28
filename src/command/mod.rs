/// A command.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
	/// The command's block.
	block: Block,

	/// The system to run the command on.
	system: System,

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
