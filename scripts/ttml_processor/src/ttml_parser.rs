use crate::types::{
    BackgroundSection, ConvertError, DefaultLanguageOptions, LyricLine, LyricSyllable,
    ParsedSourceData, RomanizationEntry, TranslationEntry,
};
use lazy_static::lazy_static;
use log::{error, warn};
use quick_xml::Reader;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use regex::Regex;
use std::collections::HashMap;
use std::str;

/// 辅助函数：解析TTML时间字符串到毫秒
pub fn parse_ttml_time_to_ms(time_str: &str) -> Result<u64, ConvertError> {
    // 例如 "12.3s"
    if let Some(stripped) = time_str.strip_suffix('s') {
        //按秒来解析
        return stripped
            .parse::<f64>()
            .map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "无法将秒值 '{}' 从时间戳 '{}' 解析为数字: {}",
                    stripped, time_str, e
                ))
            })
            .map(|seconds| (seconds * 1000.0).round() as u64);
    }

    let colon_parts: Vec<&str> = time_str.split(':').collect(); // 按冒号分割时间字符串
    let hours: u64;
    let minutes: u64;
    let seconds: u64;
    let milliseconds: u64;

    // 辅助闭包，用于解析毫秒部分，并根据其长度调整（例如 "1" -> 100ms, "12" -> 120ms, "123" -> 123ms）
    let parse_ms_part = |ms_str: &str, original_time_str: &str| -> Result<u64, ConvertError> {
        // 校验毫秒部分的长度和内容
        if ms_str.is_empty() || ms_str.len() > 3 || ms_str.chars().any(|c| !c.is_ascii_digit()) {
            return Err(ConvertError::InvalidTime(format!(
                "毫秒部分 '{}' 在时间戳 '{}' 中无效",
                ms_str, original_time_str
            )));
        }
        Ok(match ms_str.len() {
            1 => ms_str.parse::<u64>().map_err(ConvertError::ParseInt)? * 100, // 1位 -> 乘以100
            2 => ms_str.parse::<u64>().map_err(ConvertError::ParseInt)? * 10,  // 2位 -> 乘以10
            3 => ms_str.parse::<u64>().map_err(ConvertError::ParseInt)?,       // 3位 -> 直接解析
            _ => unreachable!(),                                               // 前面已经校验了长度
        })
    };

    match colon_parts.len() {
        3 => {
            // HH:MM:SS.mmm 格式
            hours = colon_parts[0].parse().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{}' 中解析小时 '{}' 失败: {}",
                    time_str, colon_parts[0], e
                ))
            })?;
            minutes = colon_parts[1].parse().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{}' 中解析分钟 '{}' 失败: {}",
                    time_str, colon_parts[1], e
                ))
            })?;
            let sec_ms_part = colon_parts[2]; // 秒和毫秒部分，例如 "SS.mmm"
            let dot_parts: Vec<&str> = sec_ms_part.split('.').collect(); // 按点分割秒和毫秒
            seconds = dot_parts[0].parse().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{}' 中解析秒 '{}' 失败: {}",
                    time_str, dot_parts[0], e
                ))
            })?;
            if dot_parts.len() == 2 {
                // 如果有毫秒部分
                milliseconds = parse_ms_part(dot_parts[1], time_str)?;
            } else if dot_parts.len() == 1 {
                // 如果只有秒部分
                milliseconds = 0;
            } else {
                // 非法格式
                return Err(ConvertError::InvalidTime(format!(
                    "在 '{}' 中秒和毫秒部分格式无效: '{}'",
                    time_str, sec_ms_part
                )));
            }
        }
        2 => {
            // MM:SS.mmm 格式
            hours = 0; // 小时为0
            minutes = colon_parts[0].parse().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{}' 中解析分钟 '{}' 失败: {}",
                    time_str, colon_parts[0], e
                ))
            })?;
            let sec_ms_part = colon_parts[1];
            let dot_parts: Vec<&str> = sec_ms_part.split('.').collect();
            seconds = dot_parts[0].parse().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{}' 中解析秒 '{}' 失败: {}",
                    time_str, dot_parts[0], e
                ))
            })?;
            if dot_parts.len() == 2 {
                milliseconds = parse_ms_part(dot_parts[1], time_str)?;
            } else if dot_parts.len() == 1 {
                milliseconds = 0;
            } else {
                return Err(ConvertError::InvalidTime(format!(
                    "在 '{}' 中秒和毫秒部分格式无效: '{}'",
                    time_str, sec_ms_part
                )));
            }
        }
        1 => {
            // SS.mmm 或 SS 格式
            hours = 0;
            minutes = 0; // 小时和分钟都为0
            let sec_ms_part = colon_parts[0];
            let dot_parts: Vec<&str> = sec_ms_part.split('.').collect();
            seconds = dot_parts[0].parse().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{}' 中解析秒 '{}' 失败: {}",
                    time_str, dot_parts[0], e
                ))
            })?;
            if dot_parts.len() == 2 {
                milliseconds = parse_ms_part(dot_parts[1], time_str)?;
            } else if dot_parts.len() == 1 {
                milliseconds = 0;
            } else {
                return Err(ConvertError::InvalidTime(format!(
                    "在 '{}' 中秒和毫秒部分格式无效: '{}'",
                    time_str, sec_ms_part
                )));
            }
        }
        _ => {
            // 其他非法格式
            return Err(ConvertError::InvalidTime(format!(
                "时间格式 '{}' 无效。",
                time_str
            )));
        }
    }
    // 校验分钟和秒是否超出范围
    if minutes >= 60 {
        return Err(ConvertError::InvalidTime(format!(
            "分钟值 '{}' (应 < 60) 在时间戳 '{}' 中无效",
            minutes, time_str
        )));
    }
    if seconds >= 60 {
        return Err(ConvertError::InvalidTime(format!(
            "秒值 '{}' (应 < 60) 在时间戳 '{}' 中无效",
            seconds, time_str
        )));
    }

    // 计算总毫秒数
    Ok(hours * 3_600_000 + minutes * 60_000 + seconds * 1000 + milliseconds)
}

#[derive(Debug, Default)]
struct TtmlParserState {
    is_line_timing_mode: bool,
    detected_line_mode: bool,
    default_main_lang: Option<String>,
    default_translation_lang: Option<String>,
    default_romanization_lang: Option<String>,
    in_metadata_section: bool,
    in_itunes_metadata: bool,
    in_am_translations: bool,
    in_am_translation: bool,
    current_am_translation_lang: Option<String>,
    translation_map: HashMap<String, (String, Option<String>)>,
    in_songwriters_tag: bool,
    in_songwriter_tag: bool,
    current_songwriter_name: String,
    in_agent_tag: bool,
    in_agent_name_tag: bool,
    current_agent_id_for_name: Option<String>,
    current_agent_name_text: String,
    in_ttm_metadata_tag: bool,
    current_ttm_metadata_key: Option<String>,
    in_body: bool,
    in_div: bool,
    in_p: bool,
    current_div_song_part: Option<String>,
    current_p_element_data: Option<CurrentPElementData>,
    span_stack: Vec<SpanContext>,
    text_buffer: String,
    last_syllable_info: LastSyllableInfo,
}

/// 存储当前处理的 `<p>` 元素解析过程中的临时数据
#[derive(Debug, Default, Clone)]
struct CurrentPElementData {
    start_ms: u64,
    end_ms: u64,
    agent: Option<String>,
    song_part: Option<String>, // 继承自 div 或 p 自身
    // 临时的行内容，最终会合并到 LyricLine
    line_text_accumulator: String, // 用于 Line 模式下的文本累积
    syllables_accumulator: Vec<LyricSyllable>, // 用于 Word 模式
    translations_accumulator: Vec<TranslationEntry>,
    romanizations_accumulator: Vec<RomanizationEntry>,
    background_section_accumulator: Option<BackgroundSectionData>,
    itunes_key: Option<String>,
}

/// 存储当前处理的 `<span ttm:role="x-bg">` 的临时数据
#[derive(Debug, Default, Clone)]
struct BackgroundSectionData {
    start_ms: u64,
    end_ms: u64,
    syllables: Vec<LyricSyllable>,
    translations: Vec<TranslationEntry>,
    romanizations: Vec<RomanizationEntry>,
}

/// 代表当前 span 的上下文，用于处理嵌套和内容类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpanRole {
    Generic,
    Translation,
    Romanization,
    Background,
}

#[derive(Debug, Clone)]
struct SpanContext {
    role: SpanRole,
    lang: Option<String>, // xml:lang 属性
    // 逐字歌词模式下 span 的时间戳信息
    start_ms: Option<u64>,
    end_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LastSyllableInfo {
    #[default]
    None,
    EndedSyllable {
        was_background: bool,
    },
}

pub fn normalize_text_whitespace(text: &str) -> String {
    let trimmed = text.trim(); // 移除首尾空格
    if trimmed.is_empty() {
        return String::new();
    }
    // 按空白分割成单词，然后用单个空格连接
    trimmed.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn process_meta_tag(
    e: &BytesStart,
    reader: &Reader<&[u8]>,
    raw_metadata: &mut HashMap<String, Vec<String>>,
) -> Result<(), ConvertError> {
    let mut key_attr = None;
    let mut value_attr = None;

    for attr_res in e.attributes() {
        let attr = attr_res?;
        let attr_key_local = attr.key.local_name();
        let attr_value_str = attr
            .decode_and_unescape_value(reader.decoder())?
            .into_owned();

        if attr_key_local.as_ref() == b"key" {
            key_attr = Some(attr_value_str);
        } else if attr_key_local.as_ref() == b"value" {
            value_attr = Some(attr_value_str);
        }
    }

    if let (Some(k), Some(v)) = (key_attr, value_attr) {
        if !k.is_empty() {
            raw_metadata.entry(k).or_default().push(v);
        }
    }
    Ok(())
}

lazy_static! {
    static ref TIMED_SPAN_RE: Regex = Regex::new(r#"<span\s+[^>]*begin\s*="#).unwrap();
}

pub fn parse_ttml_content(
    content: &str,
    default_languages: &DefaultLanguageOptions,
) -> Result<ParsedSourceData, ConvertError> {
    let has_timed_span_tags = TIMED_SPAN_RE.is_match(content);

    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);

    let mut lines: Vec<LyricLine> = Vec::new();
    let mut raw_metadata: HashMap<String, Vec<String>> = HashMap::new();
    let mut warnings: Vec<String> = Vec::new();

    // 初始化解析状态机
    let mut state = TtmlParserState {
        default_main_lang: default_languages.main.clone(),
        default_translation_lang: default_languages.translation.clone(),
        default_romanization_lang: default_languages.romanization.clone(),
        last_syllable_info: LastSyllableInfo::None,
        ..Default::default()
    };
    let mut buf = Vec::new();

    // --- 主循环，通过读取 XML 事件来驱动解析过程 ---
    loop {
        match reader.read_event_into(&mut buf) {
            // --- 处理 <tag> (非自闭合) 的开始标签 ---
            Ok(Event::Start(e)) => {
                let local_name_str = get_local_name_str_from_start(&e)?;
                match local_name_str.as_str() {
                    "meta" if state.in_metadata_section => {
                        process_meta_tag(&e, &reader, &mut raw_metadata)?;
                    }
                    "tt" => process_tt_start(
                        &e,
                        &mut state,
                        &mut raw_metadata,
                        &reader,
                        has_timed_span_tags,
                        &mut warnings,
                    )?,
                    "metadata" if !state.in_p => state.in_metadata_section = true,
                    "iTunesMetadata" if state.in_metadata_section => {
                        state.in_itunes_metadata = true
                    }
                    "translations" if state.in_itunes_metadata => {
                        state.in_am_translations = true;
                    }
                    "translation" if state.in_am_translations => {
                        state.in_am_translation = true;
                        state.current_am_translation_lang = None;
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.as_ref() == b"xml:lang" {
                                state.current_am_translation_lang =
                                    Some(attr_value_as_string(&attr, &reader)?);
                                break;
                            }
                        }
                    }
                    "text" if state.in_am_translation => {
                        let mut text_for: Option<String> = None;
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.as_ref() == b"for" {
                                text_for = Some(attr_value_as_string(&attr, &reader)?);
                                break;
                            }
                        }
                        if let Some(key) = text_for {
                            let text_content = reader.read_text(e.name())?.to_string();
                            if !text_content.is_empty() {
                                state.translation_map.insert(
                                    key,
                                    (text_content, state.current_am_translation_lang.clone()),
                                );
                            }
                        }
                    }
                    "songwriters" if state.in_itunes_metadata => state.in_songwriters_tag = true,
                    "songwriter" if state.in_songwriters_tag => {
                        state.in_songwriter_tag = true;
                        state.current_songwriter_name.clear();
                    }
                    "agent"
                        if state.in_metadata_section && e.name().as_ref().starts_with(b"ttm:") =>
                    {
                        let mut agent_id: Option<String> = None;
                        let mut agent_type: Option<String> = None;

                        for attr in e.attributes().with_checks(false).flatten() {
                            match attr.key.as_ref() {
                                b"xml:id" => agent_id = Some(attr_value_as_string(&attr, &reader)?),
                                b"type" => agent_type = Some(attr_value_as_string(&attr, &reader)?),
                                _ => {}
                            }
                        }

                        if let Some(id) = &agent_id {
                            let type_val = agent_type.unwrap_or_else(|| "person".to_string());
                            let type_key = format!("agent-type-{}", id);
                            raw_metadata.entry(type_key).or_default().push(type_val);

                            state.in_agent_tag = true;
                            state.current_agent_id_for_name = agent_id;
                        }
                    }
                    "name" if state.in_agent_tag && e.name().as_ref().starts_with(b"ttm:") => {
                        state.in_agent_name_tag = true;
                        state.current_agent_name_text.clear();
                    }
                    "body" => state.in_body = true,
                    "div" if state.in_body => {
                        state.in_div = true;
                        state.current_div_song_part = None;
                        for attr in e.attributes().with_checks(false).flatten() {
                            if attr.key.as_ref() == b"itunes:song-part" {
                                state.current_div_song_part =
                                    Some(attr_value_as_string(&attr, &reader)?);
                                break;
                            }
                        }
                    }
                    "p" if state.in_body => {
                        state.in_p = true;
                        let mut p_data = CurrentPElementData {
                            song_part: state.current_div_song_part.clone(),
                            ..Default::default()
                        };
                        for attr in e.attributes().with_checks(false).flatten() {
                            match attr.key.as_ref() {
                                b"begin" => {
                                    p_data.start_ms = parse_ttml_time_to_ms(&attr_value_as_string(
                                        &attr, &reader,
                                    )?)?
                                }
                                b"end" => {
                                    p_data.end_ms = parse_ttml_time_to_ms(&attr_value_as_string(
                                        &attr, &reader,
                                    )?)?
                                }
                                b"ttm:agent" | b"agent" => {
                                    p_data.agent = Some(attr_value_as_string(&attr, &reader)?)
                                }
                                b"itunes:song-part" => {
                                    p_data.song_part = Some(attr_value_as_string(&attr, &reader)?)
                                }
                                b"itunes:key" => {
                                    p_data.itunes_key = Some(attr_value_as_string(&attr, &reader)?);
                                }
                                _ => {}
                            }
                        }
                        state.current_p_element_data = Some(p_data);
                        state.text_buffer.clear();
                        state.span_stack.clear();
                    }
                    "span" if state.in_p => process_span_start(&e, &mut state, &reader)?,
                    _ => {
                        if state.in_metadata_section
                            && e.name().as_ref().starts_with(b"ttm:")
                            && local_name_str != "agent"
                            && local_name_str != "name"
                        {
                            state.in_ttm_metadata_tag = true;
                            state.current_ttm_metadata_key = Some(local_name_str);
                            state.text_buffer.clear();
                        }
                    }
                }
            }

            // --- 处理 <tag/> (自闭合) 标签 ---
            Ok(Event::Empty(e)) => {
                let local_name_str = get_local_name_str_from_start(&e)?;
                match local_name_str.as_str() {
                    "meta" if state.in_metadata_section => {
                        process_meta_tag(&e, &reader, &mut raw_metadata)?;
                    }
                    "agent"
                        if state.in_metadata_section && e.name().as_ref().starts_with(b"ttm:") =>
                    {
                        let mut agent_id: Option<String> = None;
                        let mut agent_type: Option<String> = None;

                        for attr_res in e.attributes() {
                            let attr = attr_res?;
                            match attr.key.as_ref() {
                                b"xml:id" => {
                                    agent_id = Some(
                                        attr.decode_and_unescape_value(reader.decoder())?
                                            .into_owned(),
                                    )
                                }
                                b"type" => {
                                    agent_type = Some(
                                        attr.decode_and_unescape_value(reader.decoder())?
                                            .into_owned(),
                                    )
                                }
                                _ => {}
                            }
                        }
                        if let Some(id) = agent_id {
                            let type_val = agent_type.unwrap_or_else(|| "person".to_string());
                            let type_key = format!("agent-type-{}", id);
                            raw_metadata.entry(type_key).or_default().push(type_val);
                        }
                    }
                    "br" if state.in_p => {
                        warnings.push(format!(
                            "在 <p> ({}ms-{}ms) 中发现并忽略了一个 <br/> 标签。",
                            state
                                .current_p_element_data
                                .as_ref()
                                .map_or(0, |d| d.start_ms),
                            state
                                .current_p_element_data
                                .as_ref()
                                .map_or(0, |d| d.end_ms)
                        ));
                    }
                    _ => {}
                }
            }

            // --- 处理文本节点 ---
            Ok(Event::Text(e)) => {
                process_text_event(e, &mut state)?;
            }

            // --- 处理 </tag> 结束标签 ---
            Ok(Event::End(e)) => {
                let ended_tag_name = get_local_name_str_from_end(&e)?;
                if ended_tag_name == "metadata" {
                    state.in_metadata_section = false;
                }

                // --- 处理不在 span 内的 <p> 标签文本 ---
                // 当一个非 span 标签结束时，检查缓冲区中是否有未处理的文本
                if ended_tag_name != "span" && !state.text_buffer.is_empty() {
                    // 如果在 <p> 标签内且不在任何 <span> 中，这部分文本属于 <p> 的直接子节点
                    if state.in_p && state.span_stack.is_empty() {
                        if let Some(p_data) = state.current_p_element_data.as_mut() {
                            // 在逐行模式下，将文本追加到行文本累加器
                            if state.is_line_timing_mode {
                                p_data.line_text_accumulator.push_str(&state.text_buffer);
                            } else if !state.text_buffer.trim().is_empty() {
                                // 在逐字模式下，<p> 内不应有无包裹的文本，发出警告
                                warn!(
                                    "TTML 逐字模式: 在 </{}> 前发现游离文本 '{}'，将被忽略。",
                                    ended_tag_name,
                                    state.text_buffer.trim().escape_debug()
                                );
                            }
                        }
                        state.text_buffer.clear();
                    } else if !state.in_ttm_metadata_tag {
                        // 如果在其他未知上下文中，发出警告
                        if !state.text_buffer.trim().is_empty() {
                            warn!(
                                "TTML解析: 在 </{}> 前发现未知上下文的游离文本 '{}'，将被忽略。",
                                ended_tag_name,
                                state.text_buffer.trim().escape_debug()
                            );
                        }
                        state.text_buffer.clear();
                    }
                }

                if state.in_ttm_metadata_tag {
                    if let Some(key) = state.current_ttm_metadata_key.as_ref() {
                        if *key == ended_tag_name {
                            let value = normalize_text_whitespace(&state.text_buffer);
                            if !value.is_empty() {
                                raw_metadata.entry(key.clone()).or_default().push(value);
                            }
                            state.in_ttm_metadata_tag = false;
                            state.current_ttm_metadata_key = None;
                            state.text_buffer.clear();
                        }
                    }
                }

                match ended_tag_name.as_str() {
                    "iTunesMetadata" if state.in_itunes_metadata => {
                        state.in_itunes_metadata = false
                    }
                    "songwriter" if state.in_songwriter_tag => {
                        if !state.current_songwriter_name.is_empty() {
                            raw_metadata
                                .entry("songwriters".to_string())
                                .or_default()
                                .push(state.current_songwriter_name.trim().to_string());
                        }
                        state.in_songwriter_tag = false;
                    }
                    "songwriters" if state.in_songwriters_tag => state.in_songwriters_tag = false,
                    "name" if state.in_agent_name_tag && e.name().as_ref().starts_with(b"ttm:") => {
                        if let Some(agent_id) = &state.current_agent_id_for_name {
                            let agent_display_name =
                                state.current_agent_name_text.trim().to_string();
                            if !agent_display_name.is_empty() {
                                raw_metadata
                                    .entry("agent".to_string())
                                    .or_default()
                                    .push(format!("{}={}", agent_id, agent_display_name));
                            }
                        }
                        state.in_agent_name_tag = false;
                    }
                    "agent" if state.in_agent_tag && e.name().as_ref().starts_with(b"ttm:") => {
                        state.in_agent_tag = false;
                        state.current_agent_id_for_name = None;
                    }
                    "div" if state.in_div => {
                        state.in_div = false;
                        state.current_div_song_part = None;
                    }
                    "p" if state.in_p => {
                        if let Some(mut p_data) = state.current_p_element_data.take() {
                            if let Some(key) = &p_data.itunes_key {
                                if let Some((text, lang)) = state.translation_map.get(key) {
                                    if p_data
                                        .translations_accumulator
                                        .iter()
                                        .all(|t| &t.text != text)
                                    {
                                        p_data.translations_accumulator.push(TranslationEntry {
                                            text: text.clone(),
                                            lang: lang.clone(),
                                        });
                                    }
                                }
                            }
                            finalize_p_element(p_data, &mut lines, &state, &mut warnings);
                        }
                        state.in_p = false;
                        state.span_stack.clear();
                        state.last_syllable_info = LastSyllableInfo::None;
                    }
                    "span" if state.in_p => process_span_end(&mut state, &mut warnings)?,
                    "translations" => state.in_am_translations = false,
                    "translation" => state.in_am_translation = false,
                    _ => {}
                }
                if !(ended_tag_name == "span"
                    && matches!(
                        state.last_syllable_info,
                        LastSyllableInfo::EndedSyllable { .. }
                    ))
                {
                    state.last_syllable_info = LastSyllableInfo::None;
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => {
                error!("TTML 解析错误，位置 {}: {}", reader.buffer_position(), e);
                return Err(ConvertError::Xml(e));
            }
            _ => {
                state.last_syllable_info = LastSyllableInfo::None;
            }
        }
        buf.clear();
    }

    Ok(ParsedSourceData {
        lines,
        raw_metadata,
        source_format: crate::types::LyricFormat::Ttml,
        source_filename: None,
        is_line_timed_source: state.is_line_timing_mode,
        warnings,
        raw_ttml_from_input: Some(content.to_string()),
        detected_formatted_ttml_input: None,
    })
}

fn get_local_name_str_from_start(e: &BytesStart) -> Result<String, ConvertError> {
    str::from_utf8(e.local_name().as_ref())
        .map(|s| s.to_string())
        .map_err(|err| ConvertError::Internal(format!("无法将标签名转换为UTF-8: {}", err)))
}

fn get_local_name_str_from_end(e: &BytesEnd) -> Result<String, ConvertError> {
    str::from_utf8(e.local_name().as_ref())
        .map(|s| s.to_string())
        .map_err(|err| ConvertError::Internal(format!("无法将标签名转换为UTF-8: {}", err)))
}

fn attr_value_as_string(attr: &Attribute, reader: &Reader<&[u8]>) -> Result<String, ConvertError> {
    Ok(attr
        .decode_and_unescape_value(reader.decoder())?
        .into_owned())
}

fn process_tt_start(
    e: &BytesStart,
    state: &mut TtmlParserState,
    raw_metadata: &mut HashMap<String, Vec<String>>,
    reader: &Reader<&[u8]>,
    has_timed_span_tags: bool,
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    let mut timing_attr_found = false;
    for attr in e.attributes().with_checks(false).flatten() {
        match attr.key.as_ref() {
            b"itunes:timing" => {
                if attr_value_as_string(&attr, reader)?.to_lowercase() == "line" {
                    state.is_line_timing_mode = true;
                }
                timing_attr_found = true;
            }
            b"xml:lang" => {
                let lang_val = attr_value_as_string(&attr, reader)?;
                if !lang_val.is_empty() {
                    raw_metadata
                        .entry("xml:lang_root".to_string())
                        .or_default()
                        .push(lang_val.clone());
                    if state.default_main_lang.is_none() {
                        state.default_main_lang = Some(lang_val);
                    }
                }
            }
            _ => {}
        }
    }

    // 如果没有找到 itunes:timing 属性，并且预扫描未发现任何带时间属性的 <span> 标签
    if !timing_attr_found && !has_timed_span_tags {
        state.is_line_timing_mode = true;
        state.detected_line_mode = true;
        warnings.push(
            "未找到带时间戳的 <span> 标签且未指定 itunes:timing 模式，已自动切换到逐行歌词模式。"
                .to_string(),
        );
    }

    Ok(())
}

fn process_span_start(
    e: &BytesStart,
    state: &mut TtmlParserState,
    reader: &Reader<&[u8]>,
) -> Result<(), ConvertError> {
    state.text_buffer.clear();

    let mut role = SpanRole::Generic;
    let mut lang = None;
    let mut start_ms = None;
    let mut end_ms = None;

    for attr in e.attributes().with_checks(false).flatten() {
        match attr.key.as_ref() {
            b"ttm:role" | b"role" => {
                let role_val = attr_value_as_string(&attr, reader)?;
                role = match role_val.as_str() {
                    "x-translation" => SpanRole::Translation,
                    "x-roman" => SpanRole::Romanization,
                    "x-bg" => SpanRole::Background,
                    _ => SpanRole::Generic,
                };
            }
            b"xml:lang" => lang = Some(attr_value_as_string(&attr, reader)?),
            b"begin" => {
                start_ms = Some(parse_ttml_time_to_ms(&attr_value_as_string(
                    &attr, reader,
                )?)?)
            }
            b"end" => {
                end_ms = Some(parse_ttml_time_to_ms(&attr_value_as_string(
                    &attr, reader,
                )?)?)
            }
            _ => {}
        }
    }
    state.span_stack.push(SpanContext {
        role,
        lang,
        start_ms,
        end_ms,
    });

    if role == SpanRole::Background {
        if let Some(p_data) = state.current_p_element_data.as_mut() {
            if p_data.background_section_accumulator.is_none() {
                p_data.background_section_accumulator = Some(BackgroundSectionData {
                    start_ms: start_ms.unwrap_or(0),
                    end_ms: end_ms.unwrap_or(0),
                    ..Default::default()
                });
            }
        }
    }
    Ok(())
}

fn process_text_event(
    e_text: quick_xml::events::BytesText,
    state: &mut TtmlParserState,
) -> Result<(), ConvertError> {
    let text_val_unescaped = e_text.unescape()?;
    let text_slice: &str = match text_val_unescaped {
        std::borrow::Cow::Borrowed(s) => s,
        std::borrow::Cow::Owned(ref s) => s.as_str(),
    };

    if state.in_songwriter_tag {
        state.current_songwriter_name.push_str(text_slice);
        return Ok(());
    }
    if state.in_agent_name_tag {
        state.current_agent_name_text.push_str(text_slice);
        return Ok(());
    }
    if state.in_ttm_metadata_tag {
        state.text_buffer.push_str(text_slice);
        return Ok(());
    }

    if state.in_p {
        let mut is_consumed_inter_syllable_space = false;

        if let LastSyllableInfo::EndedSyllable { was_background } = state.last_syllable_info {
            if !text_slice.is_empty() && text_slice.chars().all(char::is_whitespace) {
                if let Some(p_data) = state.current_p_element_data.as_mut() {
                    let target_syllables = if was_background {
                        p_data
                            .background_section_accumulator
                            .as_mut()
                            .map(|bs| &mut bs.syllables)
                    } else {
                        Some(&mut p_data.syllables_accumulator)
                    };

                    if let Some(syllables) = target_syllables {
                        if let Some(last_syl) = syllables.last_mut() {
                            if !last_syl.ends_with_space {
                                last_syl.ends_with_space = true;
                            }
                        }
                    }
                }
                is_consumed_inter_syllable_space = true;
            }
        }

        state.last_syllable_info = LastSyllableInfo::None;

        if !is_consumed_inter_syllable_space {
            if !state.span_stack.is_empty() {
                state.text_buffer.push_str(text_slice);
            } else if let Some(p_data) = state.current_p_element_data.as_mut() {
                if state.is_line_timing_mode {
                    p_data.line_text_accumulator.push_str(text_slice);
                } else {
                    if !text_slice.trim().is_empty() {
                        warn!(
                            "TTML 逐字模式: 在 <p> ({}ms) 内、<span> 标签外发现非空文本: '{}'。将被累加到临时缓冲区。",
                            p_data.start_ms,
                            text_slice.escape_debug()
                        );
                    }
                    state.text_buffer.push_str(text_slice);
                }
            }
        }
    }
    Ok(())
}

fn process_span_end(
    state: &mut TtmlParserState,
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    state.last_syllable_info = LastSyllableInfo::None;

    if let Some(ended_span_ctx) = state.span_stack.pop() {
        let raw_text_from_buffer = state.text_buffer.clone();
        state.text_buffer.clear();

        let trimmed_text_for_content_check = raw_text_from_buffer.trim();
        let had_trailing_whitespace = !raw_text_from_buffer.is_empty()
            && raw_text_from_buffer.ends_with(char::is_whitespace)
            && trimmed_text_for_content_check.len() < raw_text_from_buffer.len();

        if let Some(p_data) = state.current_p_element_data.as_mut() {
            let ended_span_was_within_background_container = state
                .span_stack
                .iter()
                .any(|s| s.role == SpanRole::Background);

            match ended_span_ctx.role {
                SpanRole::Generic => {
                    if !state.is_line_timing_mode {
                        // 音节时间戳有效性
                        if let (Some(start_ms), Some(end_ms)) =
                            (ended_span_ctx.start_ms, ended_span_ctx.end_ms)
                        {
                            // 只要音节有内容或有时长，就应该处理
                            if !trimmed_text_for_content_check.is_empty() || end_ms >= start_ms {
                                if start_ms > end_ms {
                                    warnings.push(format!("TTML解析警告: 音节 '{}' 的时间戳无效 (start_ms {} > end_ms {}), 但仍会创建音节。", trimmed_text_for_content_check.escape_debug(), start_ms, end_ms));
                                }

                                // 检查这是否是一个只包含空格的、带时长的音节
                                let is_whitespace_span =
                                    trimmed_text_for_content_check.is_empty() && end_ms >= start_ms;

                                let syllable_text: String;
                                let syllable_ends_with_space: bool;

                                if is_whitespace_span {
                                    // 这是一个有时长的空格
                                    syllable_text = " ".to_string(); // 规范化为一个空格
                                    syllable_ends_with_space = false; // 空格不应该再有尾随空格

                                    // 移除前一个音节的后缀空格（如果有）
                                    let target_syllables =
                                        if ended_span_was_within_background_container {
                                            p_data
                                                .background_section_accumulator
                                                .as_mut()
                                                .map(|bs| &mut bs.syllables)
                                        } else {
                                            Some(&mut p_data.syllables_accumulator)
                                        };

                                    if let Some(syllables) = target_syllables {
                                        if let Some(last_syl) = syllables.last_mut() {
                                            if last_syl.ends_with_space {
                                                last_syl.ends_with_space = false;
                                            }
                                        }
                                    }
                                } else {
                                    // 这是一个常规音节
                                    syllable_text = if ended_span_was_within_background_container {
                                        clean_parentheses_from_bg_text(
                                            trimmed_text_for_content_check,
                                        )
                                    } else {
                                        normalize_text_whitespace(trimmed_text_for_content_check)
                                    };
                                    syllable_ends_with_space = had_trailing_whitespace;
                                }

                                // 只有在文本不为空(包括刚刚创建的空格)的情况下才创建音节
                                if !syllable_text.is_empty() {
                                    let syllable = LyricSyllable {
                                        text: syllable_text,
                                        start_ms,
                                        end_ms: end_ms.max(start_ms),
                                        duration_ms: Some(end_ms.saturating_sub(start_ms)),
                                        ends_with_space: syllable_ends_with_space,
                                    };

                                    let was_bg_syllable =
                                        if ended_span_was_within_background_container {
                                            if let Some(bg_section) =
                                                p_data.background_section_accumulator.as_mut()
                                            {
                                                bg_section.syllables.push(syllable);
                                                true
                                            } else {
                                                false
                                            }
                                        } else {
                                            p_data.syllables_accumulator.push(syllable);
                                            false
                                        };
                                    state.last_syllable_info = LastSyllableInfo::EndedSyllable {
                                        was_background: was_bg_syllable,
                                    };
                                }
                            }
                        } else if !trimmed_text_for_content_check.is_empty() {
                            warnings.push(format!(
                                "TTML 逐字歌词下，span缺少时间信息，文本 '{}' 被忽略。",
                                trimmed_text_for_content_check.escape_debug()
                            ));
                        }
                    } else {
                        p_data.line_text_accumulator.push_str(&raw_text_from_buffer);
                    }
                }
                SpanRole::Translation | SpanRole::Romanization => {
                    let normalized_text = normalize_text_whitespace(trimmed_text_for_content_check);
                    if !normalized_text.is_empty() {
                        let lang_to_use = ended_span_ctx.lang.or_else(|| {
                            if ended_span_was_within_background_container {
                                None
                            } else if ended_span_ctx.role == SpanRole::Translation {
                                state.default_translation_lang.clone()
                            } else {
                                state.default_romanization_lang.clone()
                            }
                        });

                        if ended_span_ctx.role == SpanRole::Translation {
                            let entry = TranslationEntry {
                                text: normalized_text,
                                lang: lang_to_use,
                            };
                            if ended_span_was_within_background_container {
                                if let Some(bg_section) =
                                    p_data.background_section_accumulator.as_mut()
                                {
                                    bg_section.translations.push(entry);
                                }
                            } else {
                                p_data.translations_accumulator.push(entry);
                            }
                        } else {
                            let entry = RomanizationEntry {
                                text: normalized_text,
                                lang: lang_to_use,
                                scheme: None,
                            };
                            if ended_span_was_within_background_container {
                                if let Some(bg_section) =
                                    p_data.background_section_accumulator.as_mut()
                                {
                                    bg_section.romanizations.push(entry);
                                }
                            } else {
                                p_data.romanizations_accumulator.push(entry);
                            }
                        }
                    }
                }
                SpanRole::Background => {
                    if let Some(bg_acc) = p_data.background_section_accumulator.as_mut() {
                        if ended_span_ctx.start_ms.is_none() || ended_span_ctx.end_ms.is_none() {
                            if !bg_acc.syllables.is_empty() {
                                let min_start = bg_acc.syllables.iter().map(|s| s.start_ms).min();
                                let max_end = bg_acc.syllables.iter().map(|s| s.end_ms).max();
                                
                                if let (Some(start), Some(end)) = (min_start, max_end) {
                                    bg_acc.start_ms = start;
                                    bg_acc.end_ms = end;
                                }
                            }
                        }
                    }

                    if !trimmed_text_for_content_check.is_empty() {
                        warn!(
                            "TTML 解析警告: <span ttm:role='x-bg'> 直接包含文本 '{}'。",
                            trimmed_text_for_content_check.escape_debug()
                        );
                        if let (Some(start_ms), Some(end_ms)) =
                            (ended_span_ctx.start_ms, ended_span_ctx.end_ms)
                        {
                            if end_ms > start_ms || !trimmed_text_for_content_check.is_empty() {
                                if start_ms >= end_ms && !trimmed_text_for_content_check.is_empty()
                                {
                                    warnings.push(format!("TTML解析警告: 背景歌词文本 '{}' 的时间戳无效 (start_ms {} >= end_ms {}), 但仍会创建音节。", trimmed_text_for_content_check.escape_debug(), start_ms, end_ms));
                                }
                                if let Some(bg_acc) = p_data.background_section_accumulator.as_mut()
                                {
                                    if bg_acc.syllables.is_empty() {
                                        bg_acc.syllables.push(LyricSyllable {
                                            text: normalize_text_whitespace(
                                                trimmed_text_for_content_check,
                                            ),
                                            start_ms,
                                            end_ms: end_ms.max(start_ms),
                                            duration_ms: Some(end_ms.saturating_sub(start_ms)),
                                            ends_with_space: had_trailing_whitespace,
                                        });
                                        state.last_syllable_info =
                                            LastSyllableInfo::EndedSyllable {
                                                was_background: true,
                                            };
                                    } else {
                                        warnings.push(format!("TTML 解析警告: <span ttm:role='x-bg'> 直接包含文本 '{}'，但其内部已有音节，此直接文本被忽略。", trimmed_text_for_content_check.escape_debug()));
                                    }
                                }
                            } else {
                                warnings.push(format!("TTML 解析警告: <span ttm:role='x-bg'> 直接包含文本 '{}'，但时间戳无效，忽略。", trimmed_text_for_content_check.escape_debug()));
                            }
                        } else {
                            warnings.push(format!("TTML 解析警告: <span ttm:role='x-bg'> 直接包含文本 '{}'，但缺少时间信息，忽略。", trimmed_text_for_content_check.escape_debug()));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn finalize_p_element(
    p_data: CurrentPElementData,
    lines: &mut Vec<LyricLine>,
    state: &TtmlParserState,
    warnings: &mut Vec<String>,
) {
    let mut final_line = LyricLine {
        start_ms: p_data.start_ms,
        end_ms: p_data.end_ms,
        itunes_key: p_data.itunes_key,
        agent: p_data.agent.or_else(|| Some("v1".to_string())),
        song_part: p_data.song_part,
        translations: p_data.translations_accumulator,
        romanizations: p_data.romanizations_accumulator,
        ..Default::default()
    };

    if !state.is_line_timing_mode && !state.text_buffer.is_empty() {
        let unhandled_p_text = state.text_buffer.trim();
        if !unhandled_p_text.is_empty() {
            warnings.push(format!(
                "TTML 逐字模式警告: 段落 ({}ms-{}ms) 结束后，发现未处理的文本: '{}'。此文本被忽略。",
                p_data.start_ms, p_data.end_ms, unhandled_p_text.escape_debug()
            ));
        }
    }

    if state.is_line_timing_mode {
        let mut line_text_content = p_data.line_text_accumulator.clone();
        if line_text_content.trim().is_empty() && !p_data.syllables_accumulator.is_empty() {
            line_text_content = p_data
                .syllables_accumulator
                .iter()
                .map(|s| {
                    let mut text = s.text.clone();
                    if s.ends_with_space {
                        text.push(' ');
                    }
                    text
                })
                .collect::<String>();
            warnings.push(format!(
                "TTML解析警告: 逐行段落 ({}ms-{}ms) 的文本来自其内部的逐字结构。",
                p_data.start_ms, p_data.end_ms
            ));
        }

        let normalized_line_text_content = normalize_text_whitespace(&line_text_content);

        if !normalized_line_text_content.is_empty()
            || !final_line.translations.is_empty()
            || !final_line.romanizations.is_empty()
            || final_line.end_ms > final_line.start_ms
        {
            final_line.line_text = Some(normalized_line_text_content.clone());
            if final_line.main_syllables.is_empty() && !normalized_line_text_content.is_empty() {
                let syl_start = final_line.start_ms;
                let syl_end = final_line.end_ms;
                if syl_start > syl_end {
                    warnings.push(format!("TTML解析警告: 为行文本 '{}' 创建的音节时间戳无效 (start_ms {} > end_ms {}).", normalized_line_text_content.escape_debug(), syl_start, syl_end));
                }
                final_line.main_syllables.push(LyricSyllable {
                    text: normalized_line_text_content,
                    start_ms: syl_start,
                    end_ms: syl_end.max(syl_start),
                    duration_ms: Some(syl_end.saturating_sub(syl_start)),
                    ends_with_space: false,
                });
            }
        } else {
            return;
        }

        if !p_data.syllables_accumulator.is_empty() {
            warnings.push(format!(
                "TTML解析警告: 在逐行歌词的段落 ({}ms-{}ms) 中检测到并忽略了 {} 个逐字音节的时间戳。",
                p_data.start_ms, p_data.end_ms, p_data.syllables_accumulator.len()
            ));
        }
    } else {
        final_line.main_syllables = p_data.syllables_accumulator;
        if final_line.line_text.is_none() && !final_line.main_syllables.is_empty() {
            let mut assembled_line_text = String::new();
            for (i, syl) in final_line.main_syllables.iter().enumerate() {
                assembled_line_text.push_str(&syl.text);
                if syl.ends_with_space && i < final_line.main_syllables.len() - 1 {
                    assembled_line_text.push(' ');
                }
            }
            final_line.line_text = Some(assembled_line_text.trim_end().to_string());
        }

        if final_line.main_syllables.is_empty()
            && final_line.translations.is_empty()
            && final_line.romanizations.is_empty()
            && final_line.line_text.as_deref().is_none_or(|s| s.is_empty())
            && final_line.end_ms <= final_line.start_ms
        {
            return;
        }
    }

    if let Some(bg_data) = p_data.background_section_accumulator {
        if !bg_data.syllables.is_empty()
            || !bg_data.translations.is_empty()
            || !bg_data.romanizations.is_empty()
        {
            final_line.background_section = Some(BackgroundSection {
                start_ms: bg_data.start_ms,
                end_ms: bg_data.end_ms,
                syllables: bg_data.syllables,
                translations: bg_data.translations,
                romanizations: bg_data.romanizations,
            });
        }
    }

    if final_line.main_syllables.is_empty()
        && final_line
            .line_text
            .as_ref()
            .is_some_and(|lt| !lt.is_empty())
        && final_line.end_ms > final_line.start_ms
    {
        let syl_start = final_line.start_ms;
        let syl_end = final_line.end_ms;
        if syl_start > syl_end {
            warnings.push(format!(
                "TTML解析警告: 为行文本 '{}' 创建的音节时间戳无效 (start_ms {} > end_ms {}).",
                final_line.line_text.as_ref().unwrap().escape_debug(),
                syl_start,
                syl_end
            ));
        }
        final_line.main_syllables.push(LyricSyllable {
            text: final_line.line_text.as_ref().unwrap().clone(),
            start_ms: syl_start,
            end_ms: syl_end.max(syl_start),
            duration_ms: Some(syl_end.saturating_sub(syl_start)),
            ends_with_space: false,
        });
    }
    lines.push(final_line);
}

/// 清理文本两端的括号（单个或成对）
pub fn clean_parentheses_from_bg_text(text: &str) -> String {
    // 清理两端的空格
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "".to_string();
    }

    // 找到第一个不是括号的字符的索引
    let start_index = trimmed
        .char_indices()
        .find(|&(_i, c)| c != '(' && c != '（')
        .map(|(i, _)| i)
        // 如果所有字符都是开括号，则 find 返回 None
        // 将起始索引设为字符串末尾，以确保切片为空
        .unwrap_or_else(|| trimmed.len());

    // 找到最后一个不是括号的字符，获取其结束位置的索引
    let end_index = trimmed
        .char_indices()
        .rfind(|&(_i, c)| c != ')' && c != '）')
        .map(|(i, c)| i + c.len_utf8())
        // 如果所有字符都是闭括号，则 rfind 返回 None
        // 将结束索引设为0，以确保切片为空
        .unwrap_or(0);

    // 起始索引大于或等于结束索引，说明字符串是空的，或者只包含括号
    if start_index >= end_index {
        return "".to_string();
    }

    // 提取核心文本切片，再次清理
    trimmed[start_index..end_index].trim().to_string()
}
