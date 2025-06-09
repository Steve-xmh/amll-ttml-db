use crate::metadata_processor::MetadataStore;
use crate::ttml_parser::normalize_text_whitespace;
use crate::types::{
    BackgroundSection, CanonicalMetadataKey, ConvertError, LyricLine, RomanizationEntry,
    TranslationEntry, TtmlGenerationOptions, TtmlTimingMode,
};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

fn format_ttml_time(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;
    let millis = ms % 1000;

    if hours > 0 {
        format!("{}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    } else if minutes > 0 {
        format!("{}:{:02}.{:03}", minutes, seconds, millis)
    } else {
        format!("{}.{:03}", seconds, millis)
    }
}

pub fn generate_ttml(
    lines: &[LyricLine],
    metadata_store: &MetadataStore,
    options: &TtmlGenerationOptions,
) -> Result<String, ConvertError> {
    let mut buffer = Vec::new();
    let result = if options.format {
        let mut writer = Writer::new_with_indent(Cursor::new(&mut buffer), b' ', 2);
        generate_ttml_inner(&mut writer, lines, metadata_store, options)
    } else {
        let mut writer = Writer::new(Cursor::new(&mut buffer));
        generate_ttml_inner(&mut writer, lines, metadata_store, options)
    };

    result?;
    String::from_utf8(buffer).map_err(ConvertError::FromUtf8)
}

fn generate_ttml_inner<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    metadata_store: &MetadataStore,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    let mut tt_attributes: Vec<(&str, String)> = Vec::new();
    tt_attributes.push(("xmlns", "http://www.w3.org/ns/ttml".to_string()));
    tt_attributes.push((
        "xmlns:ttm",
        "http://www.w3.org/ns/ttml#metadata".to_string(),
    ));
    tt_attributes.push((
        "xmlns:itunes",
        "http://music.apple.com/lyric-ttml-internal".to_string(),
    ));

    let amll_keys_to_check_for_namespace = [
        CanonicalMetadataKey::Title,
        CanonicalMetadataKey::Album,
        CanonicalMetadataKey::AppleMusicId,
        CanonicalMetadataKey::Isrc,
        CanonicalMetadataKey::NcmMusicId,
        CanonicalMetadataKey::QqMusicId,
        CanonicalMetadataKey::SpotifyId,
        CanonicalMetadataKey::TtmlAuthorGithub,
        CanonicalMetadataKey::TtmlAuthorGithubLogin,
        CanonicalMetadataKey::Artist,
    ];
    if amll_keys_to_check_for_namespace
        .iter()
        .any(|key| metadata_store.get_multiple_values(key).is_some())
    {
        tt_attributes.push(("xmlns:amll", "http://www.example.com/ns/amll".to_string()));
    }

    let lang_to_use = options
        .main_language
        .as_ref()
        .or_else(|| metadata_store.get_single_value(&CanonicalMetadataKey::Language));
    if let Some(lang) = lang_to_use {
        if !lang.is_empty() {
            tt_attributes.push(("xml:lang", lang.clone()));
        }
    }

    let timing_mode_str = match options.timing_mode {
        TtmlTimingMode::Word => "Word",
        TtmlTimingMode::Line => "Line",
    };
    tt_attributes.push(("itunes:timing", timing_mode_str.to_string()));
    tt_attributes.sort_by_key(|&(key, _)| key);

    let mut tt_start_element = BytesStart::new("tt");
    for (key, value) in &tt_attributes {
        tt_start_element.push_attribute((*key, value.as_str()));
    }
    writer.write_event(Event::Start(tt_start_element))?;

    write_ttml_head(writer, metadata_store, lines, options)?;
    write_ttml_body(writer, lines, metadata_store, options)?;

    writer.write_event(Event::End(BytesEnd::new("tt")))?;
    Ok(())
}

fn write_ttml_head<W: std::io::Write>(
    writer: &mut Writer<W>,
    metadata_store: &MetadataStore,
    lines: &[LyricLine],
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    writer.write_event(Event::Start(BytesStart::new("head")))?;
    writer.write_event(Event::Start(BytesStart::new("metadata")))?;

    if options.timing_mode == TtmlTimingMode::Line {
        if !lines.is_empty() {
            let mut agent_element = BytesStart::new("ttm:agent");
            agent_element.push_attribute(("type", "person"));
            agent_element.push_attribute(("xml:id", "v1"));
            writer.write_event(Event::Empty(agent_element))?;
        }
    } else {
        let mut agent_ids_in_lines = HashSet::new();
        for line in lines {
            if let Some(agent_id) = &line.agent {
                if !agent_id.is_empty() && agent_id != "v0" {
                    agent_ids_in_lines.insert(agent_id.as_str());
                }
            }
        }
        if agent_ids_in_lines.is_empty() && !lines.is_empty() {
            let v1_defined_in_meta = metadata_store
                .get_multiple_values(&CanonicalMetadataKey::Custom("agent".to_string()))
                .is_some_and(|vals| vals.iter().any(|v| v.starts_with("v1=")))
                || metadata_store
                    .get_single_value(&CanonicalMetadataKey::Custom("v1".to_string()))
                    .is_some();

            if v1_defined_in_meta || agent_ids_in_lines.is_empty() {
                agent_ids_in_lines.insert("v1");
            }
        }

        let mut sorted_agent_ids: Vec<&str> = agent_ids_in_lines.into_iter().collect();
        sorted_agent_ids.sort_by_key(|&id| {
            id.strip_prefix('v')
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(u32::MAX)
        });

        for agent_id_str in sorted_agent_ids {
            let mut agent_element = BytesStart::new("ttm:agent");

            let agent_type_key = format!("agent-type-{}", agent_id_str);
            let agent_type = metadata_store
                .get_single_value(&CanonicalMetadataKey::Custom(agent_type_key))
                .map(|s| s.as_str())
                .unwrap_or("person");

            agent_element.push_attribute(("type", agent_type));
            agent_element.push_attribute(("xml:id", agent_id_str));

            let mut agent_name: Option<String> = None;
            if let Some(agent_entries) = metadata_store
                .get_multiple_values(&CanonicalMetadataKey::Custom("agent".to_string()))
            {
                for entry_str in agent_entries {
                    if let Some((id, name_part)) = entry_str.split_once('=') {
                        if id.trim() == agent_id_str {
                            agent_name = Some(name_part.trim().to_string());
                            break;
                        }
                    }
                }
            }
            if agent_name.is_none() {
                agent_name = metadata_store
                    .get_single_value(&CanonicalMetadataKey::Custom(agent_id_str.to_string()))
                    .cloned();
            }

            if let Some(name) = agent_name {
                if !name.is_empty() {
                    writer.write_event(Event::Start(agent_element))?;
                    writer
                        .create_element("ttm:name")
                        .write_text_content(BytesText::new(&name))?;
                    writer.write_event(Event::End(BytesEnd::new("ttm:agent")))?;
                } else {
                    writer.write_event(Event::Empty(agent_element))?;
                }
            } else {
                writer.write_event(Event::Empty(agent_element))?;
            }
        }
    }

    if let Some(songwriters_vec) =
        metadata_store.get_multiple_values(&CanonicalMetadataKey::Songwriter)
    {
        let valid_songwriters: Vec<&String> = songwriters_vec
            .iter()
            .filter(|s| !s.trim().is_empty())
            .collect();
        if !valid_songwriters.is_empty() {
            writer.write_event(Event::Start(BytesStart::from_content(
                "iTunesMetadata xmlns=\"http://music.apple.com/lyric-ttml-internal\"",
                "iTunesMetadata".len(),
            )))?;
            writer.write_event(Event::Start(BytesStart::new("songwriters")))?;
            for sw_name in valid_songwriters {
                writer
                    .create_element("songwriter")
                    .write_text_content(BytesText::new(sw_name.trim()))?;
            }
            writer.write_event(Event::End(BytesEnd::new("songwriters")))?;
            writer.write_event(Event::End(BytesEnd::new("iTunesMetadata")))?;
        }
    }

    let amll_meta_keys_to_check = [
        ("album", CanonicalMetadataKey::Album),
        ("isrc", CanonicalMetadataKey::Isrc),
        ("artists", CanonicalMetadataKey::Artist),
        ("musicName", CanonicalMetadataKey::Title),
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
                    let mut meta_element = BytesStart::new("amll:meta");
                    meta_element.push_attribute(("key", amll_key_name));
                    meta_element.push_attribute(("value", value_str.trim()));
                    writer.write_event(Event::Empty(meta_element))?;
                }
            }
        }
    }

    writer.write_event(Event::End(BytesEnd::new("metadata")))?;
    writer.write_event(Event::End(BytesEnd::new("head")))?;
    Ok(())
}

fn write_ttml_body<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    _metadata_store: &MetadataStore,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    let body_dur_ms = lines.iter().map(|line| line.end_ms).max().unwrap_or(0);
    let mut body_start_element = BytesStart::new("body");
    if body_dur_ms > 0 {
        body_start_element.push_attribute(("dur", format_ttml_time(body_dur_ms).as_str()));
    }
    writer.write_event(Event::Start(body_start_element))?;

    let mut lines_by_song_part: HashMap<Option<String>, Vec<&LyricLine>> = HashMap::new();
    for line in lines {
        lines_by_song_part
            .entry(line.song_part.clone())
            .or_default()
            .push(line);
    }
    let mut sorted_parts: Vec<(Option<String>, Vec<&LyricLine>)> = lines_by_song_part
        .into_iter()
        .filter(|(_, part_lines)| !part_lines.is_empty())
        .collect();

    sorted_parts.sort_by_key(|(_, part_lines)| {
        part_lines
            .iter()
            .map(|l| l.start_ms)
            .min()
            .unwrap_or(u64::MAX)
    });

    for (song_part_key, part_lines) in sorted_parts {
        if part_lines.is_empty() {
            continue;
        }

        let div_start_ms = part_lines.iter().map(|l| l.start_ms).min().unwrap_or(0);
        let div_end_ms = part_lines.iter().map(|l| l.end_ms).max().unwrap_or(0);
        let mut div_start_element = BytesStart::new("div");
        div_start_element.push_attribute(("begin", format_ttml_time(div_start_ms).as_str()));
        div_start_element.push_attribute(("end", format_ttml_time(div_end_ms).as_str()));
        if let Some(sp_val) = &song_part_key {
            if !sp_val.is_empty() {
                div_start_element.push_attribute(("itunes:song-part", sp_val.as_str()));
            }
        }
        writer.write_event(Event::Start(div_start_element))?;

        let mut p_key_counter = 0;
        for line in part_lines {
            p_key_counter += 1;
            let mut p_start_element = BytesStart::new("p");
            p_start_element.push_attribute(("begin", format_ttml_time(line.start_ms).as_str()));
            p_start_element.push_attribute(("end", format_ttml_time(line.end_ms).as_str()));
            p_start_element.push_attribute(("itunes:key", format!("L{}", p_key_counter).as_str()));

            if options.timing_mode == TtmlTimingMode::Line {
                if !line.line_text.as_deref().unwrap_or("").trim().is_empty()
                    || (!options.use_apple_format_rules
                        && (!line.translations.is_empty() || !line.romanizations.is_empty()))
                {
                    p_start_element.push_attribute(("ttm:agent", "v1"));
                }
            } else if let Some(agent) = &line.agent {
                if !agent.is_empty() && agent != "v0" {
                    p_start_element.push_attribute(("ttm:agent", agent.as_str()));
                } else if agent.is_empty() || agent == "v0" {
                    p_start_element.push_attribute(("ttm:agent", "v1"));
                }
            } else {
                p_start_element.push_attribute(("ttm:agent", "v1"));
            }
            writer.write_event(Event::Start(p_start_element))?;
            write_p_content(writer, line, options)?;
            writer.write_event(Event::End(BytesEnd::new("p")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("div")))?;
    }
    writer.write_event(Event::End(BytesEnd::new("body")))?;
    Ok(())
}

/// 辅助函数，用于写入翻译和罗马音的 <span> 标签。
///
/// # 参数
/// * `writer`: XML writer 的可变引用。
/// * `entries`: 要处理的数据项切片 (例如 `&[TranslationEntry]`)。
/// * `role`: 要设置的 `ttm:role` 属性值 (例如 "x-translation")。
/// * `default_lang`: 备用的默认语言代码。
/// * `get_text`: 一个闭包，用于从数据项中获取文本。
/// * `get_lang`: 一个闭包，用于从数据项中获取语言代码。
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
            let mut span = BytesStart::new("span");
            span.push_attribute(("ttm:role", role));

            // 优先使用条目自身的语言，否则使用提供的默认语言
            let lang_code = get_lang(entry).as_ref().or(default_lang.as_ref());

            if let Some(lang) = lang_code.filter(|s| !s.is_empty()) {
                span.push_attribute(("xml:lang", lang.as_str()));
            }

            writer.write_event(Event::Start(span))?;
            writer.write_event(Event::Text(BytesText::from_escaped(
                quick_xml::escape::escape(&normalized_text).as_ref(),
            )))?;
            writer.write_event(Event::End(BytesEnd::new("span")))?;
        }
    }
    Ok(())
}

fn write_p_content<W: std::io::Write>(
    writer: &mut Writer<W>,
    line: &LyricLine,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    if options.timing_mode == TtmlTimingMode::Line {
        let mut line_text_to_write = "".to_string();
        if let Some(lt) = &line.line_text {
            line_text_to_write = normalize_text_whitespace(lt); // 规范化行文本
        } else if !line.main_syllables.is_empty() {
            for (idx, syl) in line.main_syllables.iter().enumerate() {
                line_text_to_write.push_str(&syl.text); // 音节文本应已规范化
                if syl.ends_with_space && idx < line.main_syllables.len() - 1 {
                    line_text_to_write.push(' ');
                }
            }
            line_text_to_write = normalize_text_whitespace(&line_text_to_write); // 再次规范化拼接结果
        }
        if !line_text_to_write.is_empty() {
            writer.write_event(Event::Text(BytesText::from_escaped(
                quick_xml::escape::escape(&line_text_to_write).as_ref(),
            )))?;
        }
    } else {
        for (syl_idx, syl) in line.main_syllables.iter().enumerate() {
            let mut span_start_element = BytesStart::new("span");
            span_start_element.push_attribute(("begin", format_ttml_time(syl.start_ms).as_str()));
            span_start_element.push_attribute((
                "end",
                format_ttml_time(syl.end_ms.max(syl.start_ms)).as_str(),
            ));

            writer.write_event(Event::Start(span_start_element))?;
            if !syl.text.is_empty() {
                writer.write_event(Event::Text(BytesText::from_escaped(
                    quick_xml::escape::escape(&syl.text).as_ref(),
                )))?;
            }
            writer.write_event(Event::End(BytesEnd::new("span")))?;

            if syl.ends_with_space {
                let is_last_syllable_overall_in_p_content = (syl_idx == line.main_syllables.len() - 1) && // 是最后一个主音节
                    (options.use_apple_format_rules || (line.translations.is_empty() && line.romanizations.is_empty())) &&
                    (options.timing_mode == TtmlTimingMode::Line || line.background_section.is_none());

                if !is_last_syllable_overall_in_p_content {
                    if !options.format {
                        writer.write_event(Event::Text(BytesText::from_escaped(" ")))?;
                    }
                }
            }
        }
    }

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

    if options.timing_mode == TtmlTimingMode::Word {
        if let Some(bg_section) = &line.background_section {
            write_background_section(writer, bg_section, options)?;
        }
    }

    Ok(())
}

/// 写入背景歌词部分
fn write_background_section<W: std::io::Write>(
    writer: &mut Writer<W>,
    bg_section: &BackgroundSection,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    // 逐行歌词直接跳过
    if options.timing_mode == TtmlTimingMode::Line {
        return Ok(());
    }

    // 检查背景部分是否有实际内容，如果没有则提前退出
    let has_syllables = !bg_section.syllables.is_empty();
    let has_translations = !options.use_apple_format_rules
        && bg_section
            .translations
            .iter()
            .any(|t| !normalize_text_whitespace(&t.text).is_empty());
    let has_romanizations = !options.use_apple_format_rules
        && bg_section
            .romanizations
            .iter()
            .any(|r| !normalize_text_whitespace(&r.text).is_empty());

    // 如果没有任何音节、翻译、罗马音，并且时间戳无效，则不生成任何内容
    if !has_syllables
        && !has_translations
        && !has_romanizations
        && bg_section.end_ms <= bg_section.start_ms
    {
        return Ok(());
    }

    // 创建背景歌词的容器 <span> 标签
    let mut bg_container_span = BytesStart::new("span");
    bg_container_span.push_attribute(("ttm:role", "x-bg"));
    bg_container_span.push_attribute(("begin", format_ttml_time(bg_section.start_ms).as_str()));
    bg_container_span.push_attribute(("end", format_ttml_time(bg_section.end_ms).as_str()));
    writer.write_event(Event::Start(bg_container_span))?;

    // 自动为背景音节添加括号
    let num_syls = bg_section.syllables.len();
    for (idx, syl_bg) in bg_section.syllables.iter().enumerate() {
        if !syl_bg.text.is_empty() || (syl_bg.end_ms > syl_bg.start_ms) {
            // 文本本身就是空格，不加括号
            let text_to_write = if syl_bg.text.trim().is_empty() {
                syl_bg.text.clone()
            } else {
                // 根据音节位置决定如何添加括号
                match (num_syls, idx) {
                    (1, _) => format!("({})", syl_bg.text), // 只有一个音节
                    (_, 0) => format!("({}", syl_bg.text),  // 第一个音节
                    (_, i) if i == num_syls - 1 => format!("{})", syl_bg.text), // 最后一个音节
                    _ => syl_bg.text.clone(),               // 中间音节
                }
            };

            let mut span = BytesStart::new("span");
            span.push_attribute(("begin", format_ttml_time(syl_bg.start_ms).as_str()));
            span.push_attribute((
                "end",
                format_ttml_time(syl_bg.end_ms.max(syl_bg.start_ms)).as_str(),
            ));
            writer.write_event(Event::Start(span))?;
            if !text_to_write.is_empty() {
                writer.write_event(Event::Text(BytesText::from_escaped(
                    quick_xml::escape::escape(&text_to_write).as_ref(),
                )))?;
            }
            writer.write_event(Event::End(BytesEnd::new("span")))?;

            if syl_bg.ends_with_space && idx < num_syls - 1 {
                if !options.format {
                    writer.write_event(Event::Text(BytesText::from_escaped(" ")))?;
                }
            }
        }
    }

    if !options.use_apple_format_rules {
        write_auxiliary_span(
            writer,
            &bg_section.translations,
            "x-translation",
            &options.translation_language,
            |e: &TranslationEntry| &e.text,
            |e: &TranslationEntry| &e.lang,
        )?;

        write_auxiliary_span(
            writer,
            &bg_section.romanizations,
            "x-roman",
            &options.romanization_language,
            |e: &RomanizationEntry| &e.text,
            |e: &RomanizationEntry| &e.lang,
        )?;
    }

    // 关闭背景歌词容器 </span>
    writer.write_event(Event::End(BytesEnd::new("span")))?;
    Ok(())
}
