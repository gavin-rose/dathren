#![doc = include_str!("../README.md")]

mod client;
mod error;
pub mod export;
pub mod geo;
mod model;
pub mod registry;
mod source;
pub mod sources;

pub use client::Client;
pub use error::{Error, Result};
pub use model::{Caveat, DatasetMeta, Flag, Geography, Measure, Observation, Period};
pub use source::{Query, Source};
