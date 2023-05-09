// use crate::{error::Result, operation, Instance};
// use async_compression::tokio::{bufread::ZstdDecoder, write::ZstdEncoder};
// use tokio::io::AsyncRead;

// pub struct Writer {
// 	operation_hash: operation::Hash,
// 	temp_path: PathBuf,
// 	task: tokio::task::JoinHandle<Result<()>>,
// 	writer: tokio::io::DuplexStream,
// }

// impl Instance {
// 	pub async fn create_log_writer(&self, operation_hash: operation::Hash) -> Result<Writer> {
// 		// Create a temp path.
// 		let temp_path = self.temp_path();

// 		// Create a duplex stream to send logs to the task.
// 		let (writer, mut reader) = tokio::io::duplex(8192);

// 		// Create the task.
// 		let task = tokio::task::spawn({
// 			let temp_path = temp_path.clone();
// 			async move {
// 				// Open a file at the temp path.
// 				let file = tokio::fs::File::create(temp.path()).await?;

// 				// Buffer writes to the log.
// 				let writer = tokio::io::BufWriter::new(file);

// 				// Decode the log with zstd.
// 				let mut writer = ZstdEncoder::new(writer);

// 				// Copy from the reader to the writer.
// 				tokio::io::copy(&mut reader, &mut writer).await?;

// 				Ok(())
// 			}
// 		});

// 		// Create the writer.
// 		let writer = Writer {
// 			operation_hash,
// 			temp_path,
// 			task,
// 			writer,
// 		};

// 		Ok(writer)
// 	}

// 	pub async fn finalize_log_writer(&self, writer: Writer) -> Result<()> {
// 		// Drop the writer and wait for the task to complete.
// 		drop(writer.writer);
// 		writer.task.await.unwrap()?;

// 		// Create the path in the logs directory.
// 		let path = self.logs_path().join(writer.operation_hash.to_string());

// 		// Move the log to the logs directory.
// 		tokio::fs::rename(&writer.temp_path, &path).await?;

// 		Ok(())
// 	}

// 	pub async fn get_log_reader(&self, operation_hash: operation::Hash) -> Result<impl AsyncRead> {
// 		// Get the path to the log.
// 		let path = self.logs_path().join(operation_hash.to_string());

// 		// Open the log.
// 		let file = tokio::fs::File::open(path).await?;

// 		// Buffer reads from the log.
// 		let reader = tokio::io::BufReader::new(file);

// 		// Decode the log with zstd.
// 		let reader = ZstdDecoder::new(reader);

// 		Ok(reader)
// 	}
// }

// impl tokio::io::AsyncWrite for Writer {
// 	fn poll_write(
// 		mut self: std::pin::Pin<&mut Self>,
// 		cx: &mut std::task::Context<'_>,
// 		buf: &[u8],
// 	) -> std::task::Poll<Result<usize, std::io::Error>> {
// 		std::pin::Pin::new(&mut self.writer).poll_write(cx, buf)
// 	}

// 	fn poll_flush(
// 		mut self: std::pin::Pin<&mut Self>,
// 		cx: &mut std::task::Context<'_>,
// 	) -> std::task::Poll<Result<(), std::io::Error>> {
// 		std::pin::Pin::new(&mut self.writer).poll_flush(cx)
// 	}

// 	fn poll_shutdown(
// 		mut self: std::pin::Pin<&mut Self>,
// 		cx: &mut std::task::Context<'_>,
// 	) -> std::task::Poll<Result<(), std::io::Error>> {
// 		std::pin::Pin::new(&mut self.writer).poll_shutdown(cx)
// 	}
// }
