use crate::error::Result;

pub struct Writer<W> {
	_writer: W,
}

impl<W> Writer<W> {
	pub fn new(writer: W) -> Writer<W> {
		Writer { _writer: writer }
	}

	pub fn archive() -> Result<()> {
		todo!()
	}
}
