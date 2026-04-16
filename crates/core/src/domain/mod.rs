//! Domain types: decoded documents, fields, records, and coordinate tuples.

/// [`Document`](document::Document): an ordered list of decoded fields.
pub mod document;
/// [`Field`](field::Field): a single name/value pair with flags.
pub mod field;
/// [`FieldFlags`](flags::FieldFlags): bitfield describing how a field is stored/indexed.
pub mod flags;
/// [`IndexHeader`](header::IndexHeader): the 9-byte prefix of every index stream.
pub mod header;
/// [`Record`](record::Record): classified document (descriptor, group list, artifact add/remove).
pub mod record;
/// [`Uinfo`](uinfo::Uinfo): Maven coordinates decoded from a UINFO string.
pub mod uinfo;
