use anyhow::Result;
use lyrics_helper_core::{MetadataStore, TtmlGenerationOptions, TtmlParsingOptions};
use std::fs;
use ttml_processor::{generate_ttml, parse_ttml};

#[test]
fn test_parse_and_generate_ttml() -> Result<()> {
    let ttml_content = fs::read_to_string("tests/sample.ttml")?;
    let parsing_options = TtmlParsingOptions::default();
    let parsed_data = parse_ttml(&ttml_content, &parsing_options)?;

    let metadata_store = MetadataStore::from(&parsed_data);

    let agent_store = if parsed_data.agents.agents_by_id.is_empty() {
        MetadataStore::to_agent_store(&metadata_store)
    } else {
        parsed_data.agents.clone()
    };

    let generation_options = TtmlGenerationOptions {
        format: true,
        ..Default::default()
    };
    let generated_ttml = generate_ttml(
        &parsed_data.lines,
        &metadata_store,
        &agent_store,
        &generation_options,
    )?;

    fs::write("tests/output.ttml", generated_ttml)?;

    Ok(())
}
