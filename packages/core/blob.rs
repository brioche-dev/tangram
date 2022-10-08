use std::path::PathBuf;
use tokio::io::AsyncRead;

pub enum Blob {
	Local(PathBuf),
	Remote(Box<dyn AsyncRead + Unpin + Send + Sync>),
}
