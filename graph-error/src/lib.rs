//! Various errors used to implement the Graph API and OAuth clients for the graph-rs project
//! See the project on [GitHub](https://github.com/sreeise/graph-rs).

#![feature(try_trait)]
#[macro_use]
extern crate serde_derive;
#[macro_use]
pub extern crate snafu;

mod error;
mod graph_failure;
mod headers;
mod internal;
mod result;

pub use error::*;
pub use graph_failure::*;
pub use headers::*;
pub use internal::*;
pub use result::*;
