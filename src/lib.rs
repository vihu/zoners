#![doc = include_str!("../README.md")]
#![deny(unsafe_code, missing_docs, rustdoc::broken_intra_doc_links)]

mod band;
mod convert;
mod error;
mod parse;
mod places;

pub use band::Band;
pub use convert::{convert, Query, Row};
pub use error::{Error, Result};
pub use places::{load_places, parse_places, Place};
