use bytes::Bytes;
use futures::Stream;
pub use http_body_util::StreamBody;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::{Body, Frame};
use pin_project::pin_project;
use std::{
	pin::Pin,
	task::{Context, Poll},
};

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
