mod lyric_optimizer;
mod metadata_processor;
pub mod ttml_generator;
pub mod ttml_parser;
pub mod types;
pub mod validator;

pub use lyric_optimizer::apply_smoothing;
pub use metadata_processor::MetadataStore;
pub use ttml_generator::generate_ttml;
pub use ttml_parser::parse_ttml_content;
pub use types::{
    ConvertError, DefaultLanguageOptions, ParsedSourceData, TtmlGenerationOptions, TtmlTimingMode,
};
pub use validator::validate_lyrics_and_metadata;
