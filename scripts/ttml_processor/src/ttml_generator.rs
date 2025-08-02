//! # Timed Text Markup Language 歌词格式生成器
//!
//! 该解析器设计上仅用于生成 Apple Music 和 AMLL 使用的 TTML 歌词文件，
//! 无法用于生成通用的 TTML 字幕文件。

use std::{collections::HashMap, io::Cursor, sync::LazyLock};

use hyphenation::{Hyphenator, Language, Load, Standard};
use quick_xml::{
    Writer,
    events::{BytesText, Event},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    MetadataStore,
    types::{
        BackgroundSection, CanonicalMetadataKey, ConvertError, LyricLine, LyricSyllable,
        RomanizationEntry, TimedAuxiliaryLine, TranslationEntry, TtmlGenerationOptions,
        TtmlTimingMode,
    },
    utils::normalize_text_whitespace,
};

static ENGLISH_HYPHENATOR: LazyLock<Standard> = LazyLock::new(|| {
    // 从嵌入的资源中加载美式英语词典
    Standard::from_embedded(Language::EnglishUS)
        .expect("Failed to load embedded English hyphenation dictionary.")
});

/// 将毫秒时间戳格式化为 TTML 标准的时间字符串。
/// 例如：123456ms -> "2:03.456"
fn format_ttml_time(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;
    let millis = ms % 1000;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}.{millis:03}")
    } else if minutes > 0 {
        format!("{minutes}:{seconds:02}.{millis:03}")
    } else {
        format!("{seconds}.{millis:03}")
    }
}

fn write_timed_auxiliary_block<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    container_tag_name: &str,
    item_tag_name: &str,
    get_data_fn: impl Fn(&LyricLine) -> &Vec<TimedAuxiliaryLine>,
) -> Result<(), ConvertError> {
    let mut grouped_by_lang: HashMap<Option<String>, Vec<(&LyricLine, &TimedAuxiliaryLine)>> =
        HashMap::new();

    for line in lines {
        for aux_line in get_data_fn(line) {
            grouped_by_lang
                .entry(aux_line.lang.clone())
                .or_default()
                .push((line, aux_line));
        }
    }

    if grouped_by_lang.is_empty() {
        return Ok(());
    }

    writer
        .create_element(container_tag_name)
        .write_inner_content(|writer| {
            let mut sorted_groups: Vec<_> = grouped_by_lang.into_iter().collect();
            sorted_groups.sort_by_key(|(lang, _)| lang.clone());

            for (lang, entries) in sorted_groups {
                let mut item_builder = writer.create_element(item_tag_name);
                if let Some(lang_code) = lang.as_ref().filter(|s| !s.is_empty()) {
                    item_builder = item_builder.with_attribute(("xml:lang", lang_code.as_str()));
                }

                item_builder.write_inner_content(|writer| {
                    for (line, aux_line) in entries {
                        if let Some(key) = &line.itunes_key {
                            writer
                                .create_element("text")
                                .with_attribute(("for", key.as_str()))
                                .write_inner_content(|writer| {
                                    for syl in &aux_line.syllables {
                                        writer
                                            .create_element("span")
                                            .with_attribute((
                                                "begin",
                                                format_ttml_time(syl.start_ms).as_str(),
                                            ))
                                            .with_attribute((
                                                "end",
                                                format_ttml_time(syl.end_ms).as_str(),
                                            ))
                                            .write_text_content(BytesText::new(&syl.text))?;
                                    }
                                    Ok(())
                                })?;
                        }
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?;

    Ok(())
}

/// TTML 生成的主入口函数。
///
/// # 参数
/// * `lines` - 歌词行数据切片。
/// * `metadata_store` - 规范化后的元数据存储。
/// * `options` - TTML 生成选项，控制输出格式和规则。
///
/// # 返回
/// * `Ok(String)` - 成功生成的 TTML 字符串。
/// * `Err(ConvertError)` - 生成过程中发生错误。
pub fn generate_ttml(
    lines: &[LyricLine],
    metadata_store: &MetadataStore,
    options: &TtmlGenerationOptions,
) -> Result<String, ConvertError> {
    let mut buffer = Vec::new();
    let indent_char = b' ';
    let indent_size = 2;

    // 决定是否输出格式化的 TTML
    let result = if options.format {
        let mut writer =
            Writer::new_with_indent(Cursor::new(&mut buffer), indent_char, indent_size);
        generate_ttml_inner(&mut writer, lines, metadata_store, options)
    } else {
        let mut writer = Writer::new(Cursor::new(&mut buffer));
        generate_ttml_inner(&mut writer, lines, metadata_store, options)
    };

    result?;

    String::from_utf8(buffer).map_err(ConvertError::FromUtf8)
}

/// TTML 生成的核心内部逻辑。
fn generate_ttml_inner<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    metadata_store: &MetadataStore,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    let mut agent_name_to_id_map: HashMap<String, String> = HashMap::new();
    let mut next_agent_num = 1;

    const CHORUS_KEYWORDS: &[&str] = &["合", "合唱"];

    for line in lines {
        if let Some(agent_name) = line.agent.as_ref().filter(|s| !s.is_empty())
            && !agent_name_to_id_map.contains_key(agent_name)
        {
            let id_to_assign = if CHORUS_KEYWORDS.contains(&agent_name.to_lowercase().as_str()) {
                "v1000".to_string()
            } else {
                let id = format!("v{next_agent_num}");
                next_agent_num += 1;
                id
            };
            agent_name_to_id_map.insert(agent_name.clone(), id_to_assign);
        }
    }

    // 准备根元素的属性
    let mut namespace_attrs: Vec<(&str, String)> = Vec::new();
    namespace_attrs.push(("xmlns", "http://www.w3.org/ns/ttml".to_string()));
    namespace_attrs.push((
        "xmlns:ttm",
        "http://www.w3.org/ns/ttml#metadata".to_string(),
    ));
    namespace_attrs.push((
        "xmlns:itunes",
        "http://music.apple.com/lyric-ttml-internal".to_string(),
    ));

    let amll_keys_to_check_for_namespace = [
        CanonicalMetadataKey::Title,
        CanonicalMetadataKey::Artist,
        CanonicalMetadataKey::Album,
        CanonicalMetadataKey::Isrc,
        CanonicalMetadataKey::AppleMusicId,
        CanonicalMetadataKey::NcmMusicId,
        CanonicalMetadataKey::QqMusicId,
        CanonicalMetadataKey::SpotifyId,
        CanonicalMetadataKey::TtmlAuthorGithub,
        CanonicalMetadataKey::TtmlAuthorGithubLogin,
    ];
    if amll_keys_to_check_for_namespace
        .iter()
        .any(|key| metadata_store.get_multiple_values(key).is_some())
    {
        namespace_attrs.push(("xmlns:amll", "http://www.example.com/ns/amll".to_string()));
    }

    // 设置主语言属性
    let lang_attr = options
        .main_language
        .as_ref()
        .or_else(|| metadata_store.get_single_value(&CanonicalMetadataKey::Language))
        .filter(|s| !s.is_empty())
        .map(|lang| ("xml:lang", lang.clone()));

    // 设置 itunes:timing 属性
    let timing_mode_str = match options.timing_mode {
        TtmlTimingMode::Word => "Word",
        TtmlTimingMode::Line => "Line",
    };
    let timing_attr = ("itunes:timing", timing_mode_str.to_string());

    // 属性排序以保证输出稳定
    namespace_attrs.sort_by_key(|&(key, _)| key);

    // 写入 <tt> 根元素
    let mut element_writer = writer.create_element("tt");

    for (i, (key, value)) in namespace_attrs.iter().enumerate() {
        if i > 0 {
            element_writer = element_writer.new_line();
        }
        element_writer = element_writer.with_attribute((*key, value.as_str()));
    }

    element_writer = element_writer
        .new_line()
        .with_attribute((timing_attr.0, timing_attr.1.as_str()));

    if let Some((key, value)) = &lang_attr {
        element_writer = element_writer
            .new_line()
            .with_attribute((*key, value.as_str()));
    }

    element_writer.write_inner_content(|writer| {
        let to_io_err = |e: ConvertError| std::io::Error::other(e);

        // 写入 <head> 和 <body>
        write_ttml_head(
            writer,
            metadata_store,
            lines,
            options,
            &agent_name_to_id_map,
        )
        .map_err(to_io_err)?;
        write_ttml_body(writer, lines, options, &agent_name_to_id_map).map_err(to_io_err)?;

        Ok(())
    })?;

    Ok(())
}

/// 写入 TTML 的 `<head>` 部分，包含所有元数据。
fn write_ttml_head<W: std::io::Write>(
    writer: &mut Writer<W>,
    metadata_store: &MetadataStore,
    lines: &[LyricLine],
    options: &TtmlGenerationOptions,
    agent_map: &HashMap<String, String>,
) -> Result<(), ConvertError> {
    writer
        .create_element("head")
        .write_inner_content(|writer| {
            writer
                .create_element("metadata")
                .write_inner_content(|writer| {
                    let mut sorted_agents: Vec<(&String, &String)> = agent_map.iter().collect();
                    sorted_agents.sort_by_key(|&(_, v_id)| {
                        v_id.strip_prefix('v')
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(u32::MAX)
                    });

                    if agent_map.is_empty() && !lines.is_empty() {
                        writer
                            .create_element("ttm:agent")
                            .with_attribute(("type", "person"))
                            .with_attribute(("xml:id", "v1"))
                            .write_empty()?;
                    }

                    for (original_name, v_id) in sorted_agents {
                        let agent_type = if *v_id == "v1000" { "group" } else { "person" };
                        let mut element_builder = writer.create_element("ttm:agent");
                        element_builder = element_builder
                            .with_attribute(("type", agent_type))
                            .with_attribute(("xml:id", v_id.as_str()));

                        if *v_id != "v1000" && !original_name.is_empty() {
                            element_builder.write_inner_content(|writer| {
                                writer
                                    .create_element("ttm:name")
                                    .with_attribute(("type", "full"))
                                    .write_text_content(BytesText::new(original_name))?;
                                Ok(())
                            })?;
                        } else {
                            element_builder.write_empty()?;
                        }
                    }

                    let mut translations_by_lang: HashMap<Option<String>, Vec<(String, String)>> =
                        HashMap::new();
                    if options.use_apple_format_rules {
                        for line in lines {
                            if let Some(key) = &line.itunes_key {
                                for translation in &line.translations {
                                    translations_by_lang
                                        .entry(translation.lang.clone())
                                        .or_default()
                                        .push((key.clone(), translation.text.clone()));
                                }
                            }
                        }
                    }
                    let valid_songwriters: Vec<&String> = metadata_store
                        .get_multiple_values(&CanonicalMetadataKey::Songwriter)
                        .map(|vec| vec.iter().filter(|s| !s.trim().is_empty()).collect())
                        .unwrap_or_default();

                    let has_timed_translations =
                        lines.iter().any(|l| !l.timed_translations.is_empty());
                    let has_timed_romanizations =
                        lines.iter().any(|l| !l.timed_romanizations.is_empty());

                    if !translations_by_lang.is_empty()
                        || !valid_songwriters.is_empty()
                        || has_timed_translations
                        || has_timed_romanizations
                    {
                        writer
                            .create_element("iTunesMetadata")
                            .with_attribute(("xmlns", "http://music.apple.com/lyric-ttml-internal"))
                            .write_inner_content(|writer| {
                                if !translations_by_lang.is_empty() {
                                    writer.create_element("translations").write_inner_content(
                                        |writer| {
                                            for (lang, entries) in translations_by_lang {
                                                let mut trans_builder = writer
                                                    .create_element("translation")
                                                    .with_attribute(("type", "subtitle"));
                                                if let Some(lang_code) =
                                                    lang.as_ref().filter(|s| !s.is_empty())
                                                {
                                                    trans_builder = trans_builder.with_attribute((
                                                        "xml:lang",
                                                        lang_code.as_str(),
                                                    ));
                                                }
                                                trans_builder.write_inner_content(|writer| {
                                                    for (key, text) in entries {
                                                        let normalized_text =
                                                            normalize_text_whitespace(&text);
                                                        if !normalized_text.is_empty() {
                                                            writer
                                                                .create_element("text")
                                                                .with_attribute((
                                                                    "for",
                                                                    key.as_str(),
                                                                ))
                                                                .write_text_content(
                                                                    BytesText::new(
                                                                        &normalized_text,
                                                                    ),
                                                                )?;
                                                        }
                                                    }
                                                    Ok(())
                                                })?;
                                            }
                                            Ok(())
                                        },
                                    )?;
                                }

                                if !valid_songwriters.is_empty() {
                                    writer.create_element("songwriters").write_inner_content(
                                        |writer| {
                                            for sw_name in valid_songwriters {
                                                writer
                                                    .create_element("songwriter")
                                                    .write_text_content(BytesText::new(
                                                        sw_name.trim(),
                                                    ))?;
                                            }
                                            Ok(())
                                        },
                                    )?;
                                }
                                let to_io_err = |e: ConvertError| std::io::Error::other(e);

                                write_timed_auxiliary_block(
                                    writer,
                                    lines,
                                    "translations",
                                    "translation",
                                    |line| &line.timed_translations,
                                )
                                .map_err(to_io_err)?;

                                write_timed_auxiliary_block(
                                    writer,
                                    lines,
                                    "transliterations",
                                    "transliteration",
                                    |line| &line.timed_romanizations,
                                )
                                .map_err(to_io_err)?;

                                Ok(())
                            })?;
                    }

                    let amll_meta_keys_to_check = [
                        ("musicName", CanonicalMetadataKey::Title),
                        ("artists", CanonicalMetadataKey::Artist),
                        ("album", CanonicalMetadataKey::Album),
                        ("isrc", CanonicalMetadataKey::Isrc),
                        ("appleMusicId", CanonicalMetadataKey::AppleMusicId),
                        ("ncmMusicId", CanonicalMetadataKey::NcmMusicId),
                        ("spotifyId", CanonicalMetadataKey::SpotifyId),
                        ("qqMusicId", CanonicalMetadataKey::QqMusicId),
                        ("ttmlAuthorGithub", CanonicalMetadataKey::TtmlAuthorGithub),
                        (
                            "ttmlAuthorGithubLogin",
                            CanonicalMetadataKey::TtmlAuthorGithubLogin,
                        ),
                    ];
                    for (amll_key_name, canonical_key) in amll_meta_keys_to_check {
                        if let Some(values) = metadata_store.get_multiple_values(&canonical_key) {
                            for value_str in values {
                                if !value_str.trim().is_empty() {
                                    writer
                                        .create_element("amll:meta")
                                        .with_attribute(("key", amll_key_name))
                                        .with_attribute(("value", value_str.trim()))
                                        .write_empty()?;
                                }
                            }
                        }
                    }
                    Ok(())
                })?;
            Ok(())
        })?;
    Ok(())
}

/// 写入 TTML 的 <body> 部分，包含所有歌词行。
fn write_ttml_body<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    options: &TtmlGenerationOptions,
    agent_map: &HashMap<String, String>,
) -> Result<(), ConvertError> {
    let body_dur_ms = lines.iter().map(|line| line.end_ms).max().unwrap_or(0);
    let mut body_builder = writer.create_element("body");
    if body_dur_ms > 0 {
        body_builder = body_builder.with_attribute(("dur", format_ttml_time(body_dur_ms).as_str()));
    }

    if lines.is_empty() {
        body_builder.write_empty()?;
        return Ok(());
    }

    body_builder.write_inner_content(|writer| {
        let mut p_key_counter = 0;
        let mut current_div_lines: Vec<&LyricLine> = Vec::new();
        for current_line in lines {
            if current_div_lines.is_empty() {
                current_div_lines.push(current_line);
            } else {
                let prev_line = *current_div_lines.last().unwrap();
                if prev_line.song_part != current_line.song_part {
                    write_div(
                        writer,
                        &current_div_lines,
                        options,
                        &mut p_key_counter,
                        agent_map,
                    )
                    .map_err(std::io::Error::other)?;
                    current_div_lines.clear();
                }
                current_div_lines.push(current_line);
            }
        }
        if !current_div_lines.is_empty() {
            write_div(
                writer,
                &current_div_lines,
                options,
                &mut p_key_counter,
                agent_map,
            )
            .map_err(std::io::Error::other)?;
        }
        Ok(())
    })?;
    Ok(())
}

/// 将歌词行写入一个 div 块
fn write_div<W: std::io::Write>(
    writer: &mut Writer<W>,
    part_lines: &[&LyricLine],
    options: &TtmlGenerationOptions,
    p_key_counter: &mut i32,
    agent_map: &HashMap<String, String>,
) -> Result<(), ConvertError> {
    if part_lines.is_empty() {
        return Ok(());
    }

    let div_start_ms = part_lines.first().unwrap().start_ms;
    let div_end_ms = part_lines
        .iter()
        .map(|l| l.end_ms)
        .max()
        .unwrap_or(div_start_ms);
    let song_part_key = &part_lines.first().unwrap().song_part;

    let mut div_builder = writer.create_element("div");
    div_builder = div_builder
        .with_attribute(("begin", format_ttml_time(div_start_ms).as_str()))
        .with_attribute(("end", format_ttml_time(div_end_ms).as_str()));

    if let Some(sp_val) = song_part_key.as_ref().filter(|s| !s.is_empty()) {
        div_builder = div_builder.with_attribute(("itunes:song-part", sp_val.as_str()));
    }

    div_builder.write_inner_content(|writer| {
        for line in part_lines {
            *p_key_counter += 1;
            let agent_id_to_set = line
                .agent
                .as_ref()
                .and_then(|name| agent_map.get(name))
                .map_or("v1", |id| id.as_str());

            writer
                .create_element("p")
                .with_attribute(("begin", format_ttml_time(line.start_ms).as_str()))
                .with_attribute(("end", format_ttml_time(line.end_ms).as_str()))
                .with_attribute(("itunes:key", format!("L{p_key_counter}").as_str()))
                .with_attribute(("ttm:agent", agent_id_to_set))
                .write_inner_content(|writer| {
                    write_p_content(writer, line, options).map_err(std::io::Error::other)
                })?;
        }
        Ok(())
    })?;
    Ok(())
}

/// 根据选项写入音节，如果启用了自动分词则先进行分词。
fn write_syllable_with_optional_splitting<W: std::io::Write>(
    writer: &mut Writer<W>,
    syl: &LyricSyllable,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    if options.auto_word_splitting && syl.text.trim().chars().count() > 1 {
        let tokens = auto_tokenize(&syl.text);

        let last_visible_token_index = tokens.iter().rposition(|token| {
            get_char_type(token.chars().next().unwrap_or(' ')) != CharType::Whitespace
        });

        let total_weight: f64 = tokens
            .iter()
            .map(|token| {
                let first_char = token.chars().next().unwrap_or(' ');
                match get_char_type(first_char) {
                    CharType::Latin | CharType::Numeric | CharType::Cjk => {
                        token.chars().count() as f64
                    }
                    CharType::Other => options.punctuation_weight,
                    CharType::Whitespace => 0.0,
                }
            })
            .sum();

        if total_weight > 0.0 {
            let total_duration = syl.end_ms.saturating_sub(syl.start_ms);
            let duration_per_weight = total_duration as f64 / total_weight;

            let mut current_token_start_ms = syl.start_ms;
            let mut accumulated_weight = 0.0;

            for (token_idx, token) in tokens.iter().enumerate() {
                let first_char = token.chars().next().unwrap_or(' ');
                let char_type = get_char_type(first_char);

                if char_type == CharType::Whitespace {
                    continue;
                }

                let token_weight = match char_type {
                    CharType::Latin | CharType::Numeric | CharType::Cjk => {
                        token.chars().count() as f64
                    }
                    CharType::Other => options.punctuation_weight,
                    _ => 0.0,
                };

                accumulated_weight += token_weight;

                let mut token_end_ms =
                    syl.start_ms + (accumulated_weight * duration_per_weight).round() as u64;

                if Some(token_idx) == last_visible_token_index {
                    token_end_ms = syl.end_ms;
                }

                let text_to_write = if options.format
                    && syl.ends_with_space
                    && Some(token_idx) == last_visible_token_index
                {
                    format!("{token} ")
                } else {
                    token.to_string()
                };

                writer
                    .create_element("span")
                    .with_attribute(("begin", format_ttml_time(current_token_start_ms).as_str()))
                    .with_attribute(("end", format_ttml_time(token_end_ms).as_str()))
                    .write_text_content(BytesText::new(&text_to_write))?;

                current_token_start_ms = token_end_ms;
            }
        } else {
            write_single_syllable_span(writer, syl, options)?;
        }
    } else {
        write_single_syllable_span(writer, syl, options)?;
    }
    Ok(())
}

/// 写入 <p> 标签的具体内容，包括主歌词、翻译、罗马音和背景人声。
fn write_p_content<W: std::io::Write>(
    writer: &mut Writer<W>,
    line: &LyricLine,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    // 根据计时模式写入主歌词
    if options.timing_mode == TtmlTimingMode::Line {
        // 逐行模式直接写入整行文本
        let line_text_to_write = line
            .line_text
            .as_ref()
            .map(|s| normalize_text_whitespace(s))
            .unwrap_or_default();
        if !line_text_to_write.is_empty() {
            writer.write_event(Event::Text(BytesText::new(&line_text_to_write)))?;
        }
    } else {
        // 逐字模式：为每个音节创建带时间戳的 <span>
        for (syl_idx, syl) in line.main_syllables.iter().enumerate() {
            write_syllable_with_optional_splitting(writer, syl, options)?;

            if syl.ends_with_space && syl_idx < line.main_syllables.len() - 1 && !options.format {
                writer.write_event(Event::Text(BytesText::new(" ")))?;
            }
        }
    }

    // 仅当不使用 Apple 规则时，才生成内嵌的 `<span>` 翻译和罗马音
    if !options.use_apple_format_rules {
        write_auxiliary_span(
            writer,
            &line.translations,
            "x-translation",
            &options.translation_language,
            |e: &TranslationEntry| &e.text,
            |e: &TranslationEntry| &e.lang,
        )?;
        write_auxiliary_span(
            writer,
            &line.romanizations,
            "x-roman",
            &options.romanization_language,
            |e: &RomanizationEntry| &e.text,
            |e: &RomanizationEntry| &e.lang,
        )?;
    }

    // 仅在逐字模式下，才写入背景人声部分
    if options.timing_mode == TtmlTimingMode::Word
        && let Some(bg_section) = &line.background_section
    {
        write_background_section(writer, bg_section, options)?;
    }

    Ok(())
}

/// 写入背景歌词部分 (`<span ttm:role="x-bg">...</span>`)。
fn write_background_section<W: std::io::Write>(
    writer: &mut Writer<W>,
    bg_section: &BackgroundSection,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    if options.timing_mode == TtmlTimingMode::Line
        || (bg_section.syllables.is_empty() && bg_section.end_ms <= bg_section.start_ms)
    {
        return Ok(());
    }

    writer
        .create_element("span")
        .with_attribute(("ttm:role", "x-bg"))
        .with_attribute(("begin", format_ttml_time(bg_section.start_ms).as_str()))
        .with_attribute(("end", format_ttml_time(bg_section.end_ms).as_str()))
        .write_inner_content(|writer| {
            let num_syls = bg_section.syllables.len();
            for (idx, syl_bg) in bg_section.syllables.iter().enumerate() {
                if syl_bg.text.is_empty() && syl_bg.end_ms <= syl_bg.start_ms {
                    continue;
                }

                let text_to_write = if syl_bg.text.trim().is_empty() {
                    syl_bg.text.clone()
                } else {
                    match (num_syls, idx) {
                        (1, _) => format!("({})", syl_bg.text),
                        (_, 0) => format!("({}", syl_bg.text),
                        (_, i) if i == num_syls - 1 => format!("{})", syl_bg.text),
                        _ => syl_bg.text.clone(),
                    }
                };

                let temp_syl = LyricSyllable {
                    text: text_to_write,
                    ..syl_bg.clone()
                };

                write_syllable_with_optional_splitting(writer, &temp_syl, options)
                    .map_err(std::io::Error::other)?;

                if syl_bg.ends_with_space && idx < num_syls - 1 && !options.format {
                    writer.write_event(Event::Text(BytesText::new(" ")))?;
                }
            }

            if !options.use_apple_format_rules {
                let to_io_err = |e: ConvertError| std::io::Error::other(e);
                write_auxiliary_span(
                    writer,
                    &bg_section.translations,
                    "x-translation",
                    &options.translation_language,
                    |e| &e.text,
                    |e| &e.lang,
                )
                .map_err(to_io_err)?;
                write_auxiliary_span(
                    writer,
                    &bg_section.romanizations,
                    "x-roman",
                    &options.romanization_language,
                    |e| &e.text,
                    |e| &e.lang,
                )
                .map_err(to_io_err)?;
            }
            Ok(())
        })?;
    Ok(())
}

/// 写入单个音节span
fn write_single_syllable_span<W: std::io::Write>(
    writer: &mut Writer<W>,
    syl: &LyricSyllable,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    let text_to_write = if options.format && syl.ends_with_space {
        format!("{} ", syl.text)
    } else {
        syl.text.clone()
    };

    writer
        .create_element("span")
        .with_attribute(("begin", format_ttml_time(syl.start_ms).as_str()))
        .with_attribute((
            "end",
            format_ttml_time(syl.end_ms.max(syl.start_ms)).as_str(),
        ))
        .write_text_content(BytesText::new(&text_to_write))?;
    Ok(())
}

/// 辅助函数，用于写入翻译和罗马音的 `<span>` 标签。
fn write_auxiliary_span<T, W: std::io::Write>(
    writer: &mut Writer<W>,
    entries: &[T],
    role: &str,
    default_lang: &Option<String>,
    get_text: impl Fn(&T) -> &String,
    get_lang: impl Fn(&T) -> &Option<String>,
) -> Result<(), ConvertError> {
    for entry in entries {
        let text = get_text(entry);
        let normalized_text = normalize_text_whitespace(text);
        if !normalized_text.is_empty() {
            let mut element_builder = writer
                .create_element("span")
                .with_attribute(("ttm:role", role));

            let lang_code = get_lang(entry).as_ref().or(default_lang.as_ref());
            if let Some(lang) = lang_code.filter(|s| !s.is_empty()) {
                element_builder = element_builder.with_attribute(("xml:lang", lang.as_str()));
            }

            element_builder.write_text_content(BytesText::new(&normalized_text))?;
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum CharType {
    Cjk,
    Latin,
    Numeric,
    Whitespace,
    Other,
}

fn get_char_type(c: char) -> CharType {
    if c.is_whitespace() {
        CharType::Whitespace
    } else if c.is_ascii_alphabetic() {
        CharType::Latin
    } else if c.is_ascii_digit() {
        CharType::Numeric
    } else if (0x4E00..=0x9FFF).contains(&(c as u32))
        || (0x3040..=0x309F).contains(&(c as u32))
        || (0x30A0..=0x30FF).contains(&(c as u32))
        || (0xAC00..=0xD7AF).contains(&(c as u32))
    {
        CharType::Cjk
    } else {
        CharType::Other
    }
}

fn auto_tokenize(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut last_char_type: Option<CharType> = None;

    for grapheme in text.graphemes(true) {
        let first_char = grapheme.chars().next().unwrap_or(' ');
        let current_char_type = get_char_type(first_char);

        if let Some(last_type) = last_char_type {
            let should_break = !matches!(
                (last_type, current_char_type),
                (CharType::Latin, CharType::Latin)
                    | (CharType::Numeric, CharType::Numeric)
                    | (CharType::Whitespace, CharType::Whitespace)
            );

            if should_break && !current_token.is_empty() {
                // 如果刚刚结束的 token 是一个拉丁词，并且长度大于1，就尝试按音节拆分
                if last_type == CharType::Latin && current_token.chars().count() > 1 {
                    // 拆分为多个部分
                    tokens.extend(
                        ENGLISH_HYPHENATOR
                            .hyphenate(&current_token)
                            .into_iter()
                            .segments()
                            .map(String::from),
                    );
                } else {
                    // 对于非拉丁词（如数字、单个字符）或未拆分的词，直接推入
                    tokens.push(current_token);
                }
                current_token = String::new();
            }
        }
        current_token.push_str(grapheme);
        last_char_type = Some(current_char_type);
    }

    // 处理循环结束后的最后一个 token
    if !current_token.is_empty() {
        if last_char_type == Some(CharType::Latin) && current_token.chars().count() > 1 {
            tokens.extend(
                ENGLISH_HYPHENATOR
                    .hyphenate(&current_token)
                    .into_iter()
                    .segments()
                    .map(String::from),
            );
        } else {
            tokens.push(current_token);
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ttml_time() {
        assert_eq!(format_ttml_time(3723456), "1:02:03.456");
        assert_eq!(format_ttml_time(310100), "5:10.100");
        assert_eq!(format_ttml_time(7123), "7.123");
        assert_eq!(format_ttml_time(0), "0.000");
        assert_eq!(format_ttml_time(59999), "59.999");
        assert_eq!(format_ttml_time(60000), "1:00.000");
    }

    #[test]
    fn test_auto_tokenize() {
        assert_eq!(auto_tokenize("Hello world"), vec!["Hello", " ", "world"]);
        assert_eq!(auto_tokenize("你好世界"), vec!["你", "好", "世", "界"]);
        assert_eq!(auto_tokenize("Hello你好"), vec!["Hello", "你", "好"]);
        assert_eq!(auto_tokenize("word123"), vec!["word", "123"]);
        assert_eq!(
            auto_tokenize("你好-世界"),
            vec!["你", "好", "-", "世", "界"]
        );
        assert_eq!(auto_tokenize("Hello  world"), vec!["Hello", "  ", "world"]);
        assert_eq!(auto_tokenize(""), Vec::<String>::new());
        assert_eq!(
            auto_tokenize("OK, Let's GO! 走吧123"),
            vec![
                "OK", ",", " ", "Let", "'", "s", " ", "GO", "!", " ", "走", "吧", "123"
            ]
        );
    }

    #[test]
    fn test_auto_tokenize_with_syllables() {
        assert_eq!(
            auto_tokenize("hyphenation"),
            vec!["hy", "phen", "a", "tion"]
        );
        assert_eq!(auto_tokenize("Amazing!"), vec!["Amaz", "ing", "!",]);
        assert_eq!(
            auto_tokenize("wonderful世界"),
            vec!["won", "der", "ful", "世", "界"]
        );
    }
}
