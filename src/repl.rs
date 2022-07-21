use crate::id::Id;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Repl(pub Id);
