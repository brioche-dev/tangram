#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod client;
pub mod expression;
pub mod hash;
mod heuristics;
pub mod id;
pub mod lockfile;
pub mod manifest;
pub mod package;
pub mod server;
pub mod specifier;
pub mod system;
mod util;
