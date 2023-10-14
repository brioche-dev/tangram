use bytes::Bytes;
use tangram_client as tg;

pub trait Progress: Send + 'static {
	fn clone_box(&self) -> Box<dyn Progress>;
	fn child(&self, child: tg::Build);
	fn log(&self, bytes: Bytes);
	fn output(&self, output: Option<tg::Value>);
}
