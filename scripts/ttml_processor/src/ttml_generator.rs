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

/// 将毫秒时间戳格式化为 TTML 标准的时间字符串。
/// 例如：123456ms -> "2:03.456"
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
    // 决定是否输出格式化的 TTML
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

/// TTML 生成的核心内部逻辑。
fn generate_ttml_inner<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    metadata_store: &MetadataStore,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    // --- 准备 <tt> 根元素的属性 ---
    let mut namespace_attrs: Vec<(&str, String)> = Vec::new();
    let timing_attr;
    let mut lang_attr: Option<(&str, String)> = None;

    namespace_attrs.push(("xmlns", "http://www.w3.org/ns/ttml".to_string()));
    namespace_attrs.push((
        "xmlns:ttm",
        "http://www.w3.org/ns/ttml#metadata".to_string(),
    ));
    namespace_attrs.push((
        "xmlns:itunes",
        "http://music.apple.com/lyric-ttml-internal".to_string(),
    ));

    // 检查是否需要 amll 命名空间
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
        namespace_attrs.push(("xmlns:amll", "http://www.example.com/ns/amll".to_string()));
    }

    // 设置主语言属性
    let lang_to_use = options
        .main_language
        .as_ref()
        .or_else(|| metadata_store.get_single_value(&CanonicalMetadataKey::Language));
    if let Some(lang) = lang_to_use {
        if !lang.is_empty() {
            lang_attr = Some(("xml:lang", lang.clone()));
        }
    }

    // 设置 itunes:timing 属性
    let timing_mode_str = match options.timing_mode {
        TtmlTimingMode::Word => "Word",
        TtmlTimingMode::Line => "Line",
    };
    timing_attr = Some(("itunes:timing", timing_mode_str.to_string()));

    // 属性排序以保证输出稳定
    namespace_attrs.sort_by_key(|&(key, _)| key);

    // --- 写入 <tt> 根元素 ---
    let mut tt_start_element = BytesStart::new("tt");

    for (key, value) in &namespace_attrs {
        tt_start_element.push_attribute((*key, value.as_str()));
    }

    if let Some((key, value)) = &timing_attr {
        tt_start_element.push_attribute((*key, value.as_str()));
    }

    if let Some((key, value)) = &lang_attr {
        tt_start_element.push_attribute((*key, value.as_str()));
    }
    writer.write_event(Event::Start(tt_start_element))?;

    // --- 写入 <head> 和 <body> ---
    write_ttml_head(writer, metadata_store, lines, options)?;
    write_ttml_body(writer, lines, metadata_store, options)?;

    writer.write_event(Event::End(BytesEnd::new("tt")))?;
    Ok(())
}

/// 写入 TTML 的 <head> 部分，包含所有元数据。
fn write_ttml_head<W: std::io::Write>(
    writer: &mut Writer<W>,
    metadata_store: &MetadataStore,
    lines: &[LyricLine],
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    writer.write_event(Event::Start(BytesStart::new("head")))?;
    writer.write_event(Event::Start(BytesStart::new("metadata")))?;

    // --- 写入 ttm:agent 元数据 (演唱者信息) ---
    if options.timing_mode == TtmlTimingMode::Line {
        if !lines.is_empty() {
            let mut agent_element = BytesStart::new("ttm:agent");
            agent_element.push_attribute(("type", "person"));
            agent_element.push_attribute(("xml:id", "v1"));
            writer.write_event(Event::Empty(agent_element))?;
        }
    } else {
        // 对于逐字模式，收集所有在歌词行中实际使用到的 agent ID
        let mut agent_ids_in_lines = HashSet::new();
        for line in lines {
            if let Some(agent_id) = &line.agent {
                if !agent_id.is_empty() && agent_id != "v0" {
                    agent_ids_in_lines.insert(agent_id.as_str());
                }
            }
        }
        // 如果没有指定任何 agent，则默认定义一个 "v1"
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

        // 遍历并写入每个 agent 的定义
        for agent_id_str in sorted_agent_ids {
            let mut agent_element = BytesStart::new("ttm:agent");
            let agent_type_key = format!("agent-type-{}", agent_id_str);
            let agent_type = metadata_store
                .get_single_value(&CanonicalMetadataKey::Custom(agent_type_key))
                .map(|s| s.as_str())
                .unwrap_or("person");
            agent_element.push_attribute(("type", agent_type));
            agent_element.push_attribute(("xml:id", agent_id_str));
            // 查找 agent 的显示名称
            let agent_name = metadata_store
                .get_multiple_values(&CanonicalMetadataKey::Custom("agent".to_string()))
                .and_then(|vals| {
                    vals.iter().find_map(|v| {
                        v.strip_prefix(&format!("{}=", agent_id_str))
                            .map(str::to_string)
                    })
                });
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

    // --- 处理 Apple Music 特定元数据 (`iTunesMetadata`) ---

    // 1. 如果启用了 Apple 格式规则，则收集所有翻译数据
    let mut translations_by_lang: HashMap<Option<String>, Vec<(String, String)>> = HashMap::new();
    if options.use_apple_format_rules {
        for line in lines {
            if let Some(key) = &line.itunes_key {
                // 行必须有 itunes:key
                if !line.translations.is_empty() {
                    for translation in &line.translations {
                        // 按语言对翻译进行分组
                        translations_by_lang
                            .entry(translation.lang.clone())
                            .or_default()
                            .push((key.clone(), translation.text.clone()));
                    }
                }
            }
        }
    }

    // 2. 收集词曲作者信息
    let valid_songwriters: Vec<&String> = metadata_store
        .get_multiple_values(&CanonicalMetadataKey::Songwriter)
        .map(|vec| vec.iter().filter(|s| !s.trim().is_empty()).collect())
        .unwrap_or_default();

    // 3. 只有在需要写入翻译 或 词曲作者信息时，才创建 <iTunesMetadata> 容器
    if !translations_by_lang.is_empty() || !valid_songwriters.is_empty() {
        writer.write_event(Event::Start(BytesStart::from_content(
            "iTunesMetadata xmlns=\"http://music.apple.com/lyric-ttml-internal\"",
            "iTunesMetadata".len(),
        )))?;

        // 3a. 写入 Apple 格式的翻译
        if !translations_by_lang.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("translations")))?;
            for (lang, entries) in translations_by_lang {
                let mut trans_tag = BytesStart::new("translation");
                trans_tag.push_attribute(("type", "subtitle"));
                if let Some(lang_code) = lang.filter(|s| !s.is_empty()) {
                    // 过滤掉空语言代码
                    trans_tag.push_attribute(("xml:lang", lang_code.as_str()));
                }
                writer.write_event(Event::Start(trans_tag))?;

                for (key, text) in entries {
                    let normalized_text = normalize_text_whitespace(&text);
                    if !normalized_text.is_empty() {
                        writer
                            .create_element("text")
                            .with_attribute(("for", key.as_str()))
                            .write_text_content(BytesText::new(&normalized_text))?;
                    }
                }
                writer.write_event(Event::End(BytesEnd::new("translation")))?;
            }
            writer.write_event(Event::End(BytesEnd::new("translations")))?;
        }

        // 3b. 写入词曲作者
        if !valid_songwriters.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("songwriters")))?;
            for sw_name in valid_songwriters {
                writer
                    .create_element("songwriter")
                    .write_text_content(BytesText::new(sw_name.trim()))?;
            }
            writer.write_event(Event::End(BytesEnd::new("songwriters")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("iTunesMetadata")))?;
    }

    // --- 写入 amll:meta 自定义元数据 ---
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

/// 写入 TTML 的 <body> 部分，包含所有歌词行。
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

    // 按 song_part (如 "Verse", "Chorus") 对歌词行进行分组，以生成 <div> 结构
    let mut lines_by_song_part: HashMap<Option<String>, Vec<&LyricLine>> = HashMap::new();
    for line in lines {
        lines_by_song_part
            .entry(line.song_part.clone())
            .or_default()
            .push(line);
    }
    // 转换为 Vec 并排序，确保 <div> 的输出顺序与歌曲时间线一致
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

    // 遍历每一个 part (<div>)
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

        // p_key_counter 用于生成 `itunes:key="L1", "L2", ...` 属性
        let mut p_key_counter = 0;
        // 遍历 <div> 内的每一行 (<p>)
        for line in part_lines {
            p_key_counter += 1;
            let mut p_start_element = BytesStart::new("p");
            p_start_element.push_attribute(("begin", format_ttml_time(line.start_ms).as_str()));
            p_start_element.push_attribute(("end", format_ttml_time(line.end_ms).as_str()));
            p_start_element.push_attribute(("itunes:key", format!("L{}", p_key_counter).as_str()));

            // 为 <p> 标签设置演唱者 agent
            if options.timing_mode == TtmlTimingMode::Line {
                p_start_element.push_attribute(("ttm:agent", "v1"));
            } else if let Some(agent) = line.agent.as_ref().filter(|a| !a.is_empty() && *a != "v0")
            {
                p_start_element.push_attribute(("ttm:agent", agent.as_str()));
            } else {
                p_start_element.push_attribute(("ttm:agent", "v1")); // 默认或备用 agent
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
            writer.write_event(Event::Text(BytesText::from_escaped(
                quick_xml::escape::escape(&line_text_to_write).as_ref(),
            )))?;
        }
    } else {
        // 逐字模式：为每个音节创建带时间戳的 <span>
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
            // 如果音节后需要空格，并且不是最后一个可见元素，则写入一个空格
            if syl.ends_with_space && syl_idx < line.main_syllables.len() - 1 {
                if !options.format {
                    writer.write_event(Event::Text(BytesText::from_escaped(" ")))?;
                }
            }
        }
    }

    // --- 核心逻辑：条件性地写入内联翻译和罗马音 ---
    // 仅当不使用 Apple 规则时，才生成内联的 `<span>` 翻译和罗马音
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
    if options.timing_mode == TtmlTimingMode::Word {
        if let Some(bg_section) = &line.background_section {
            write_background_section(writer, bg_section, options)?;
        }
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
        return Ok(()); // 逐行模式或空内容时直接跳过
    }

    let mut bg_container_span = BytesStart::new("span");
    bg_container_span.push_attribute(("ttm:role", "x-bg"));
    bg_container_span.push_attribute(("begin", format_ttml_time(bg_section.start_ms).as_str()));
    bg_container_span.push_attribute(("end", format_ttml_time(bg_section.end_ms).as_str()));
    writer.write_event(Event::Start(bg_container_span))?;

    // 自动为背景音节添加括号
    let num_syls = bg_section.syllables.len();
    for (idx, syl_bg) in bg_section.syllables.iter().enumerate() {
        if !syl_bg.text.is_empty() || (syl_bg.end_ms > syl_bg.start_ms) {
            let text_to_write = if syl_bg.text.trim().is_empty() {
                syl_bg.text.clone() // 如果只是空格，则不加括号
            } else {
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
