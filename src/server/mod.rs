use crate::{
	error::{Error, Result},
	instance::Instance,
};
use bytes::Bytes;
use futures::{FutureExt, Stream};
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::{Body, Frame};
use itertools::Itertools;
use pin_project::pin_project;
use std::{
	convert::Infallible,
	net::SocketAddr,
	pin::Pin,
	task::{Context, Poll},
};

mod block;
mod error;

#[derive(Clone)]
pub struct Server {
	tg: Instance,
}

impl Server {
	#[must_use]
	pub fn new(tg: Instance) -> Self {
		Self { tg }
	}

	pub async fn serve(self, addr: SocketAddr) -> Result<()> {
		let listener = tokio::net::TcpListener::bind(&addr)
			.await
			.map_err(Error::other)?;
		tracing::info!("🚀 Serving on {}.", addr);
		loop {
			let (stream, _) = listener.accept().await?;
			let stream = TokioIo::new(stream);
			let server = self.clone();
			tokio::spawn(async move {
				hyper::server::conn::http1::Builder::new()
					.serve_connection(
						stream,
						hyper::service::service_fn(move |request| {
							let server = server.clone();
							async move {
								let response = server.handle_request(request).await;
								Ok::<_, Infallible>(response)
							}
						}),
					)
					.await
					.ok()
			});
		}
	}

	async fn handle_request(&self, request: http::Request<Incoming>) -> http::Response<Outgoing> {
		match self.handle_request_inner(request).await {
			Ok(Some(response)) => response,
			Ok(None) => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(full("Not found."))
				.unwrap(),
			Err(error) => {
				tracing::error!(?error);
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(full("Internal server error."))
					.unwrap()
			},
		}
	}

	async fn handle_request_inner(
		&self,
		request: http::Request<Incoming>,
	) -> Result<Option<http::Response<Outgoing>>> {
		let method = request.method().clone();
		let path = request.uri().path().to_owned();
		let path_components = path.split('/').skip(1).collect_vec();
		let response = match (method, path_components.as_slice()) {
			(http::Method::GET, ["v1", "blocks", _]) => {
				Some(self.handle_get_block_request(request).boxed())
			},
			(http::Method::PUT, ["v1", "blocks", _]) => {
				Some(self.handle_put_block_request(request).boxed())
			},
			// (http::Method::POST, ["v1", "operations", _, "evaluate"]) => {
			// 	Some(self.handle_operation_evaluate_request(request).boxed())
			// },
			// (http::Method::GET, ["v1", "evaluations", _]) => {
			// 	Some(self.handle_get_evaluation_request(request).boxed())
			// },
			// (http::Method::GET, ["v1", "evaluations", _, "log"]) => {
			// 	Some(self.handle_get_evaluation_request(request).boxed())
			// },
			// (http::Method::POST, ["v1", "commands", _, "execute"]) => {
			// 	Some(self.handle_command_execute_request(request).boxed())
			// },
			(_, _) => None,
		};
		let response = if let Some(response) = response {
			Some(response.await.map_err(Error::other)?)
		} else {
			None
		};
		Ok(response)
	}
}

pub type Incoming = hyper::body::Incoming;
pub type Outgoing = BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[must_use]
pub fn empty() -> Outgoing {
	Empty::new().map_err(|_| unreachable!()).boxed()
}

pub fn full(chunk: impl Into<Bytes>) -> Outgoing {
	Full::new(chunk.into()).map_err(|_| unreachable!()).boxed()
}

#[pin_project]
pub struct BodyStream<B> {
	#[pin]
	body: B,
}

impl<B> BodyStream<B> {
	pub fn new(body: B) -> BodyStream<B> {
		BodyStream { body }
	}
}

impl<B> Body for BodyStream<B>
where
	B: Body,
{
	type Data = B::Data;
	type Error = B::Error;

	fn poll_frame(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
		self.project().body.poll_frame(cx)
	}
}

impl<B> Stream for BodyStream<B>
where
	B: Body,
{
	type Item = Result<Frame<B::Data>, B::Error>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		match self.project().body.poll_frame(cx) {
			Poll::Ready(Some(frame)) => Poll::Ready(Some(frame)),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

#[derive(Debug)]
#[pin_project]
pub struct TokioIo<T> {
	#[pin]
	inner: T,
}

impl<T> TokioIo<T> {
	pub fn new(inner: T) -> Self {
		Self { inner }
	}

	pub fn inner(self) -> T {
		self.inner
	}
}

impl<T> hyper::rt::Read for TokioIo<T>
where
	T: tokio::io::AsyncRead,
{
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		mut buf: hyper::rt::ReadBufCursor<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		let n = unsafe {
			let mut buf = tokio::io::ReadBuf::uninit(buf.as_mut());
			match tokio::io::AsyncRead::poll_read(self.project().inner, cx, &mut buf) {
				Poll::Ready(Ok(())) => buf.filled().len(),
				other => return other,
			}
		};
		unsafe {
			buf.advance(n);
		}
		Poll::Ready(Ok(()))
	}
}

impl<T> hyper::rt::Write for TokioIo<T>
where
	T: tokio::io::AsyncWrite,
{
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<Result<usize, std::io::Error>> {
		tokio::io::AsyncWrite::poll_write(self.project().inner, cx, buf)
	}

	fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
		tokio::io::AsyncWrite::poll_flush(self.project().inner, cx)
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		tokio::io::AsyncWrite::poll_shutdown(self.project().inner, cx)
	}

	fn is_write_vectored(&self) -> bool {
		tokio::io::AsyncWrite::is_write_vectored(&self.inner)
	}

	fn poll_write_vectored(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		bufs: &[std::io::IoSlice<'_>],
	) -> Poll<Result<usize, std::io::Error>> {
		tokio::io::AsyncWrite::poll_write_vectored(self.project().inner, cx, bufs)
	}
}

impl<T> tokio::io::AsyncRead for TokioIo<T>
where
	T: hyper::rt::Read,
{
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		let filled = buf.filled().len();
		let sub_filled = unsafe {
			let mut buf = hyper::rt::ReadBuf::uninit(buf.unfilled_mut());
			match hyper::rt::Read::poll_read(self.project().inner, cx, buf.unfilled()) {
				Poll::Ready(Ok(())) => buf.filled().len(),
				other => return other,
			}
		};
		let n_filled = filled + sub_filled;
		let n_init = sub_filled;
		unsafe {
			buf.assume_init(n_init);
			buf.set_filled(n_filled);
		}
		Poll::Ready(Ok(()))
	}
}

impl<T> tokio::io::AsyncWrite for TokioIo<T>
where
	T: hyper::rt::Write,
{
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		buf: &[u8],
	) -> Poll<Result<usize, std::io::Error>> {
		hyper::rt::Write::poll_write(self.project().inner, cx, buf)
	}

	fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
		hyper::rt::Write::poll_flush(self.project().inner, cx)
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		hyper::rt::Write::poll_shutdown(self.project().inner, cx)
	}

	fn is_write_vectored(&self) -> bool {
		hyper::rt::Write::is_write_vectored(&self.inner)
	}

	fn poll_write_vectored(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		bufs: &[std::io::IoSlice<'_>],
	) -> Poll<Result<usize, std::io::Error>> {
		hyper::rt::Write::poll_write_vectored(self.project().inner, cx, bufs)
	}
}
