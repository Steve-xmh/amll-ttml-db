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
    metadata_processor::MetadataStore,
    types::{
        Agent, AgentStore, AgentType, AnnotatedTrack, CanonicalMetadataKey, ContentType,
        ConvertError, LyricLine, LyricSyllable, LyricTrack, TrackMetadataKey,
        TtmlGenerationOptions, TtmlTimingMode,
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

fn write_timed_tracks_to_head<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    p_key_counter_base: i32,
    track_kind: &str, // "translation" 或 "romanization"
    container_tag_name: &str,
    item_tag_name: &str,
) -> Result<(), ConvertError> {
    // 按语言对轨道进行分组
    let mut grouped_by_lang: HashMap<Option<String>, Vec<(i32, &LyricTrack)>> = HashMap::new();

    for (line_idx, line) in lines.iter().enumerate() {
        for annotated_track in &line.tracks {
            let tracks_to_check = match track_kind {
                "translation" => &annotated_track.translations,
                "romanization" => &annotated_track.romanizations,
                _ => continue,
            };

            for track in tracks_to_check {
                if track
                    .words
                    .iter()
                    .any(|w| w.syllables.iter().any(|s| s.end_ms > s.start_ms))
                {
                    let lang = track.metadata.get(&TrackMetadataKey::Language).cloned();
                    let line_key = line_idx.try_into().unwrap_or(i32::MAX - p_key_counter_base)
                        + p_key_counter_base;
                    grouped_by_lang
                        .entry(lang)
                        .or_default()
                        .push((line_key, track));
                }
            }
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
                    for (line_idx, track) in entries {
                        writer
                            .create_element("text")
                            .with_attribute(("for", format!("L{line_idx}").as_str()))
                            .write_inner_content(|writer| {
                                for word in &track.words {
                                    for syl in &word.syllables {
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
                                }
                                Ok(())
                            })?;
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
    agent_store: &AgentStore,
    options: &TtmlGenerationOptions,
) -> Result<String, ConvertError> {
    let mut buffer = Vec::new();
    let indent_char = b' ';
    let indent_size = 2;

    // 决定是否输出格式化的 TTML
    let result = if options.format {
        let mut writer =
            Writer::new_with_indent(Cursor::new(&mut buffer), indent_char, indent_size);
        generate_ttml_inner(&mut writer, lines, metadata_store, agent_store, options)
    } else {
        let mut writer = Writer::new(Cursor::new(&mut buffer));
        generate_ttml_inner(&mut writer, lines, metadata_store, agent_store, options)
    };

    result?;

    String::from_utf8(buffer).map_err(ConvertError::FromUtf8)
}

/// TTML 生成的核心内部逻辑。
fn generate_ttml_inner<W: std::io::Write>(
    writer: &mut Writer<W>,
    lines: &[LyricLine],
    metadata_store: &MetadataStore,
    agent_store: &AgentStore,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
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
        write_ttml_head(writer, metadata_store, lines, agent_store, options)?;
        write_ttml_body(writer, lines, options)?;
        Ok(())
    })?;

    Ok(())
}

fn write_ttml_head<W: std::io::Write>(
    writer: &mut Writer<W>,
    metadata_store: &MetadataStore,
    lines: &[LyricLine],
    agent_store: &AgentStore,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    writer
        .create_element("head")
        .write_inner_content(|writer| {
            writer
                .create_element("metadata")
                .write_inner_content(|writer| {
                    let mut sorted_agents: Vec<_> = agent_store.all_agents().cloned().collect();

                    if sorted_agents.is_empty() && !lines.is_empty() {
                        // 如果没有 agent 但有歌词行，创建一个默认的
                        sorted_agents.push(Agent {
                            id: "v1".to_string(),
                            name: None,
                            agent_type: AgentType::Person,
                        });
                    }

                    sorted_agents.sort_by(|a, b| a.id.cmp(&b.id));

                    for agent in sorted_agents {
                        let type_str = match agent.agent_type {
                            AgentType::Person => "person",
                            AgentType::Group => "group",
                            AgentType::Other => "other",
                        };

                        let agent_element = writer
                            .create_element("ttm:agent")
                            .with_attribute(("type", type_str))
                            .with_attribute(("xml:id", agent.id.as_str()));

                        if let Some(name) = &agent.name {
                            agent_element.write_inner_content(|writer| {
                                writer
                                    .create_element("ttm:name")
                                    .with_attribute(("type", "full"))
                                    .write_text_content(BytesText::new(name))?;
                                Ok(())
                            })?;
                        } else {
                            agent_element.write_empty()?;
                        }
                    }

                    if options.use_apple_format_rules {
                        let mut translations_by_lang: HashMap<
                            Option<String>,
                            Vec<(String, String)>,
                        > = HashMap::new();

                        for (i, line) in lines.iter().enumerate() {
                            let p_key = format!("L{}", i + 1);
                            for at in &line.tracks {
                                for track in &at.translations {
                                    let all_syllables: Vec<_> =
                                        track.words.iter().flat_map(|w| &w.syllables).collect();
                                    let is_timed =
                                        all_syllables.iter().any(|s| s.end_ms > s.start_ms);

                                    if !is_timed || all_syllables.len() <= 1 {
                                        let lang = track
                                            .metadata
                                            .get(&TrackMetadataKey::Language)
                                            .cloned();
                                        let full_text = all_syllables
                                            .iter()
                                            .map(|s| s.text.clone())
                                            .collect::<Vec<_>>()
                                            .join(" ");

                                        if !full_text.trim().is_empty() {
                                            translations_by_lang.entry(lang).or_default().push((
                                                p_key.clone(),
                                                normalize_text_whitespace(&full_text),
                                            ));
                                        }
                                    }
                                }
                            }
                        }

                        let valid_songwriters: Vec<&String> = metadata_store
                            .get_multiple_values(&CanonicalMetadataKey::Songwriter)
                            .map(|vec| vec.iter().filter(|s| !s.trim().is_empty()).collect())
                            .unwrap_or_default();

                        // 检查是否有任何内容来证明创建 <iTunesMetadata> 块是合理的
                        let has_timed_translations = lines.iter().any(|l| {
                            l.tracks.iter().any(|at| {
                                at.translations
                                    .iter()
                                    .any(|t| t.words.iter().flat_map(|w| &w.syllables).count() > 1)
                            })
                        });
                        let has_timed_romanizations = lines
                            .iter()
                            .any(|l| l.tracks.iter().any(|at| !at.romanizations.is_empty()));

                        if !translations_by_lang.is_empty()
                            || !valid_songwriters.is_empty()
                            || has_timed_translations
                            || has_timed_romanizations
                        {
                            writer
                                .create_element("iTunesMetadata")
                                .with_attribute((
                                    "xmlns",
                                    "http://music.apple.com/lyric-ttml-internal",
                                ))
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
                                                        trans_builder = trans_builder
                                                            .with_attribute((
                                                                "xml:lang",
                                                                lang_code.as_str(),
                                                            ));
                                                    }
                                                    trans_builder.write_inner_content(
                                                        |writer| {
                                                            for (key, text) in entries {
                                                                writer
                                                                    .create_element("text")
                                                                    .with_attribute((
                                                                        "for",
                                                                        key.as_str(),
                                                                    ))
                                                                    .write_text_content(
                                                                        BytesText::new(&text),
                                                                    )?;
                                                            }
                                                            Ok(())
                                                        },
                                                    )?;
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

                                    write_timed_tracks_to_head(
                                        writer,
                                        lines,
                                        1,
                                        "translation",
                                        "translations",
                                        "translation",
                                    )
                                    .map_err(to_io_err)?;

                                    write_timed_tracks_to_head(
                                        writer,
                                        lines,
                                        1,
                                        "romanization",
                                        "transliterations",
                                        "transliteration",
                                    )
                                    .map_err(to_io_err)?;

                                    Ok(())
                                })?;
                        }
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
                    write_div(writer, &current_div_lines, options, &mut p_key_counter)
                        .map_err(std::io::Error::other)?;
                    current_div_lines.clear();
                }
                current_div_lines.push(current_line);
            }
        }
        if !current_div_lines.is_empty() {
            write_div(writer, &current_div_lines, options, &mut p_key_counter)
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

            let agent_id_to_set = line.agent.as_deref().unwrap_or("v1");

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
                        let char_count = token.chars().count();
                        let safe_count: u32 = char_count.try_into().unwrap_or(1_000_000);
                        f64::from(safe_count)
                    }
                    CharType::Other => options.punctuation_weight,
                    CharType::Whitespace => 0.0,
                }
            })
            .sum();

        if total_weight > 0.0 {
            let total_duration = syl.end_ms.saturating_sub(syl.start_ms);
            let safe_duration: u32 = total_duration.try_into().unwrap_or(2_000_000_000);
            let duration_per_weight = f64::from(safe_duration) / total_weight;

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
                        let char_count = token.chars().count();
                        let safe_count: u32 = char_count.try_into().unwrap_or(1_000_000);
                        f64::from(safe_count)
                    }
                    CharType::Other => options.punctuation_weight,
                    CharType::Whitespace => 0.0,
                };

                accumulated_weight += token_weight;

                let offset_ms = (accumulated_weight * duration_per_weight).round();
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let safe_offset = if (0.0..=1_000_000_000.0).contains(&offset_ms) {
                    offset_ms as u64
                } else if offset_ms > 1_000_000_000.0 {
                    1_000_000_000
                } else {
                    0
                };
                let mut token_end_ms = syl.start_ms.saturating_add(safe_offset);

                if Some(token_idx) == last_visible_token_index {
                    token_end_ms = syl.end_ms;
                }

                let text_to_write = if options.format
                    && syl.ends_with_space
                    && Some(token_idx) == last_visible_token_index
                {
                    format!("{token} ")
                } else {
                    token.clone()
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
    let main_content_tracks: Vec<_> = line
        .tracks
        .iter()
        .filter(|at| at.content_type == ContentType::Main)
        .collect();
    let background_annotated_tracks: Vec<_> = line
        .tracks
        .iter()
        .filter(|at| at.content_type == ContentType::Background)
        .collect();

    // 1. 处理主内容
    if options.timing_mode == TtmlTimingMode::Line {
        let line_text_to_write = main_content_tracks
            .iter()
            .flat_map(|at| at.content.words.iter().flat_map(|w| &w.syllables))
            .map(|syl| syl.text.clone())
            .collect::<Vec<_>>()
            .join(if options.format { " " } else { "" });

        if !line_text_to_write.is_empty() {
            writer.write_event(Event::Text(BytesText::new(&normalize_text_whitespace(
                &line_text_to_write,
            ))))?;
        }
    } else {
        for at in &main_content_tracks {
            write_track_as_spans(writer, &at.content, options)?;
        }
    }

    // 2. 处理内联辅助轨道 (用于主内容轨道)
    if !options.use_apple_format_rules {
        for at in &main_content_tracks {
            for track in &at.translations {
                write_inline_auxiliary_track(writer, track, "x-translation", options)?;
            }
            for track in &at.romanizations {
                write_inline_auxiliary_track(writer, track, "x-roman", options)?;
            }
        }
    }

    // 3. 处理背景内容
    if options.timing_mode == TtmlTimingMode::Word && !background_annotated_tracks.is_empty() {
        write_background_tracks(writer, &background_annotated_tracks, options)?;
    }

    Ok(())
}

fn write_inline_auxiliary_track<W: std::io::Write>(
    writer: &mut Writer<W>,
    track: &LyricTrack,
    role: &str,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    let mut element_builder = writer
        .create_element("span")
        .with_attribute(("ttm:role", role));

    if let Some(lang) = track.metadata.get(&TrackMetadataKey::Language)
        && !lang.is_empty()
    {
        element_builder = element_builder.with_attribute(("xml:lang", lang.as_str()));
    }

    let all_syllables: Vec<_> = track.words.iter().flat_map(|w| &w.syllables).collect();
    if all_syllables.is_empty() {
        return Ok(());
    }

    let is_timed = all_syllables.iter().any(|s| s.end_ms > s.start_ms);
    let has_multiple_syllables = all_syllables.len() > 1;

    let write_as_nested_timed_spans =
        is_timed && options.timing_mode == TtmlTimingMode::Word && has_multiple_syllables;

    if write_as_nested_timed_spans {
        let start_ms = all_syllables.iter().map(|s| s.start_ms).min().unwrap_or(0);
        let end_ms = all_syllables.iter().map(|s| s.end_ms).max().unwrap_or(0);

        element_builder
            .with_attribute(("begin", format_ttml_time(start_ms).as_str()))
            .with_attribute(("end", format_ttml_time(end_ms).as_str()))
            .write_inner_content(|writer| {
                write_track_as_spans(writer, track, options).map_err(std::io::Error::other)
            })?;
    } else {
        let full_text = all_syllables
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<_>>()
            .join(if options.format { " " } else { "" });

        let normalized_text = normalize_text_whitespace(&full_text);
        if !normalized_text.is_empty() {
            element_builder.write_text_content(BytesText::new(&normalized_text))?;
        }
    }

    Ok(())
}

fn write_track_as_spans<W: std::io::Write>(
    writer: &mut Writer<W>,
    track: &LyricTrack,
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    let all_syllables: Vec<_> = track.words.iter().flat_map(|w| &w.syllables).collect();
    for (syl_idx, syl) in all_syllables.iter().enumerate() {
        write_syllable_with_optional_splitting(writer, syl, options)?;

        if syl.ends_with_space && syl_idx < all_syllables.len() - 1 && !options.format {
            writer.write_event(Event::Text(BytesText::new(" ")))?;
        }
    }
    Ok(())
}

fn write_background_tracks<W: std::io::Write>(
    writer: &mut Writer<W>,
    bg_annotated_tracks: &[&AnnotatedTrack],
    options: &TtmlGenerationOptions,
) -> Result<(), ConvertError> {
    let all_syls: Vec<_> = bg_annotated_tracks
        .iter()
        .flat_map(|at| at.content.words.iter().flat_map(|w| &w.syllables))
        .collect();
    if all_syls.is_empty() {
        return Ok(());
    }

    let start_ms = all_syls.iter().map(|s| s.start_ms).min().unwrap_or(0);
    let end_ms = all_syls.iter().map(|s| s.end_ms).max().unwrap_or(0);

    writer
        .create_element("span")
        .with_attribute(("ttm:role", "x-bg"))
        .with_attribute(("begin", format_ttml_time(start_ms).as_str()))
        .with_attribute(("end", format_ttml_time(end_ms).as_str()))
        .write_inner_content(|writer| {
            let num_syls = all_syls.len();
            for (idx, syl_bg) in all_syls.iter().enumerate() {
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
                    ..(*syl_bg).clone()
                };

                write_syllable_with_optional_splitting(writer, &temp_syl, options)
                    .map_err(std::io::Error::other)?;

                if syl_bg.ends_with_space && idx < num_syls - 1 && !options.format {
                    writer.write_event(Event::Text(BytesText::new(" ")))?;
                }
            }
            for at in bg_annotated_tracks {
                for track in &at.translations {
                    write_inline_auxiliary_track(writer, track, "x-translation", options)
                        .map_err(std::io::Error::other)?;
                }
                for track in &at.romanizations {
                    write_inline_auxiliary_track(writer, track, "x-roman", options)
                        .map_err(std::io::Error::other)?;
                }
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
        assert_eq!(format_ttml_time(3_723_456), "1:02:03.456");
        assert_eq!(format_ttml_time(310_100), "5:10.100");
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
