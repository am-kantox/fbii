pub mod model;
pub mod parser;

pub use model::{OpdsEntry, OpdsFeed, OpdsLinkType};
pub use parser::{parse_opds_feed, resolve_url};
