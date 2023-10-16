use bytes::Bytes;
use tangram_client as tg;
use tg::Result;

pub trait Progress: Send + Sync + 'static {
	fn clone_box(&self) -> Box<dyn Progress>;
	fn child(&self, child: &tg::Build);
	fn log(&self, bytes: Bytes);
	fn result(&self, output: Result<tg::Value>);
}
