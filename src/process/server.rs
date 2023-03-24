use crate::{error::Result, Instance};
use tokio::net::UnixListener;

pub struct Server<'a> {
	tg: &'a Instance,
}

impl<'a> Server<'a> {
	pub async fn serve(self) -> Result<()> {
		todo!()
		// let listener = UnixListener::bind(path).await?;
		// println!("Listening on http://{}", addr);
		// loop {
		// 	let (stream, _) = listener.accept().await?;
		// 	tokio::task::spawn(async move {
		// 		if let Err(err) = hyper::server::conn::http1::Builder::new()
		// 			.serve_connection(stream, hyper::service::service_fn(hello))
		// 			.await
		// 		{
		// 			println!("Error serving connection: {:?}", err);
		// 		}
		// 	});
		// }
	}
}
