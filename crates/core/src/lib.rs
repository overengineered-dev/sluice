//! Pure-Rust parser for the Maven Central binary index format.
//!
//! The crate is I/O-neutral: it operates on any `std::io::Read` and has no
//! knowledge of gzip, files, HTTP, or JSON. Gzip wrapping and output belong
//! to the CLI crate.

#![allow(clippy::missing_errors_doc)]

pub mod domain;
pub mod error;
pub mod parser;

pub use domain::{
    document::Document,
    field::Field,
    flags::FieldFlags,
    header::IndexHeader,
    record::{classify, Record},
    uinfo::Uinfo,
};
pub use error::ParseError;
pub use parser::IndexReader;
