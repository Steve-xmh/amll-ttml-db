mod metadata_processor;
pub mod ttml_parser;
pub mod types;
pub mod validator;
pub mod ttml_generator;

pub use metadata_processor::MetadataStore;
pub use types::{ParsedSourceData, TtmlGenerationOptions, TtmlTimingMode, ConvertError, DefaultLanguageOptions};
pub use ttml_parser::parse_ttml_content;
pub use validator::validate_lyrics_and_metadata;
pub use ttml_generator::generate_ttml;