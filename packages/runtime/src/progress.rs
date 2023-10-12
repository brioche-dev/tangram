use tangram_client as tg;

pub trait Progress: Send + 'static {
	fn clone_box(&self) -> Box<dyn Progress>;
	fn child(&self, child: tg::Build);
	fn log(&self, bytes: Vec<u8>);
	fn output(&self, output: Option<tg::Value>);
}
