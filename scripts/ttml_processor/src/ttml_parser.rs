//! # TTML (Timed Text Markup Language) 解析器
//!
//! 该解析器设计上仅用于解析 Apple Music 和 AMLL 使用的 TTML 歌词文件，
//! 不建议用于解析通用的 TTML 字幕文件。

use std::{collections::HashMap, str};

use quick_xml::{
    Reader,
    de::from_str,
    errors::Error as QuickXmlError,
    events::{BytesStart, BytesText, Event},
};
use serde::Deserialize;
use tracing::{error, warn};

use crate::types::{
    BackgroundSection, ConvertError, LyricFormat, LyricLine, LyricSyllable, ParsedSourceData,
    RomanizationEntry, TranslationEntry, TtmlParsingOptions, TtmlTimingMode,
};

// =================================================================================
// 1. 常量定义
// =================================================================================

const TAG_TT: &[u8] = b"tt";
const TAG_METADATA: &[u8] = b"metadata";
const TAG_BODY: &[u8] = b"body";
const TAG_DIV: &[u8] = b"div";
const TAG_P: &[u8] = b"p";
const TAG_SPAN: &[u8] = b"span";
const TAG_BR: &[u8] = b"br";

const ATTR_ITUNES_TIMING: &[u8] = b"itunes:timing";
const ATTR_XML_LANG: &[u8] = b"xml:lang";
const ATTR_ITUNES_SONG_PART: &[u8] = b"itunes:song-part";
const ATTR_BEGIN: &[u8] = b"begin";
const ATTR_END: &[u8] = b"end";
const ATTR_AGENT: &[u8] = b"ttm:agent";
const ATTR_AGENT_ALIAS: &[u8] = b"agent";
const ATTR_ITUNES_KEY: &[u8] = b"itunes:key";
const ATTR_ROLE: &[u8] = b"ttm:role";
const ATTR_ROLE_ALIAS: &[u8] = b"role";
const ATTR_XML_SCHEME: &[u8] = b"xml:scheme";

const ROLE_TRANSLATION: &[u8] = b"x-translation";
const ROLE_ROMANIZATION: &[u8] = b"x-roman";
const ROLE_BACKGROUND: &[u8] = b"x-bg";

// =================================================================================
// 2. 状态机和元数据结构体
// =================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatDetection {
    Undetermined,
    IsFormatted,
    NotFormatted,
}

impl Default for FormatDetection {
    fn default() -> Self {
        Self::Undetermined
    }
}

/// 主解析器状态机，聚合了所有子状态和全局配置。
#[derive(Debug, Default)]
struct TtmlParserState {
    // --- 全局配置与状态 ---
    /// 是否为逐行计时模式。由 `<tt itunes:timing="line">` 或自动检测确定。
    is_line_timing_mode: bool,
    /// 标记是否是通过启发式规则（没有找到带时间的span）自动检测为逐行模式。
    detected_line_mode: bool,
    /// 标记是否被检测为格式化的 TTML（包含大量换行和缩进）。
    format_detection: FormatDetection,
    /// 用于格式化检测的计数器。
    whitespace_nodes_with_newline: u32,
    /// 已处理的节点总数，用于格式化检测。
    total_nodes_processed: u32,
    /// 默认的主要语言。
    default_main_lang: Option<String>,
    /// 默认的翻译语言。
    default_translation_lang: Option<String>,
    /// 默认的罗马音语言。
    default_romanization_lang: Option<String>,
    /// 通用文本缓冲区，用于临时存储标签内的文本内容。
    text_buffer: String,
    /// 文本处理缓冲区，用于优化字符串处理。
    text_processing_buffer: String,

    // --- 子状态机 ---
    /// 存储 `<metadata>` 区域解析状态的结构体。
    metadata_state: MetadataParseState,
    /// 存储 `<body>` 和 `<p>` 区域解析状态的结构体。
    body_state: BodyParseState,
}

/// 存储 `<metadata>` 区域解析状态的结构体。
#[derive(Debug, Default)]
struct MetadataParseState {
    /// 存储从 `<iTunesMetadata>` 解析出的翻译，key 是 itunes:key。
    translation_map: HashMap<String, (String, Option<String>)>,
    /// 存储从 Agent ID 到 Agent Name 的映射。
    agent_id_to_name_map: HashMap<String, String>,
}

/// 存储 `<body>` 和 `<p>` 区域解析状态的结构体。
#[derive(Debug, Default)]
struct BodyParseState {
    in_body: bool,
    in_div: bool,
    in_p: bool,
    /// 当前 `<div>` 的 `itunes:song-part` 属性，会被子 `<p>` 继承。
    current_div_song_part: Option<String>,
    /// 存储当前正在处理的 `<p>` 元素的临时数据。
    current_p_element_data: Option<CurrentPElementData>,
    /// `<span>` 标签的上下文堆栈，用于处理嵌套的 span。
    span_stack: Vec<SpanContext>,
    /// 记录上一个处理的音节信息，主要用于判断音节间的空格。
    last_syllable_info: LastSyllableInfo,
}

/// 存储当前处理的 `<p>` 元素解析过程中的临时数据。
#[derive(Debug, Default)]
struct CurrentPElementData {
    start_ms: u64,
    end_ms: u64,
    agent: Option<String>,
    song_part: Option<String>, // 继承自 div 或 p 自身
    itunes_key: Option<String>,
    /// 用于在逐行模式下累积所有文本内容。
    line_text_accumulator: String,
    /// 用于在逐字模式下累积所有音节。
    syllables_accumulator: Vec<LyricSyllable>,
    /// 用于累积当前行内的所有翻译。
    translations_accumulator: Vec<TranslationEntry>,
    /// 用于累积当前行内的所有罗马音。
    romanizations_accumulator: Vec<RomanizationEntry>,
    /// 用于累积当前行内的背景人声部分。
    background_section_accumulator: Option<BackgroundSectionData>,
}

/// 存储当前处理的 `<span ttm:role="x-bg">` 临时数据。
#[derive(Debug, Default, Clone)]
struct BackgroundSectionData {
    start_ms: u64,
    end_ms: u64,
    syllables: Vec<LyricSyllable>,
    translations: Vec<TranslationEntry>,
    romanizations: Vec<RomanizationEntry>,
}

/// 代表当前 `<span>` 的上下文信息，用于处理嵌套和内容分类。
#[derive(Debug, Clone)]
struct SpanContext {
    role: SpanRole,
    lang: Option<String>,   // xml:lang 属性
    scheme: Option<String>, // xml:scheme 属性
    start_ms: Option<u64>,
    end_ms: Option<u64>,
}

/// 定义 `<span>` 标签可能扮演的角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpanRole {
    /// 普通音节
    Generic,
    /// 翻译    
    Translation,
    /// 罗马音
    Romanization,
    /// 背景人声容器
    Background,
}

/// 记录最后一个结束的音节信息，用于正确处理音节间的空格。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LastSyllableInfo {
    #[default]
    /// 初始状态或上一个不是音节
    None,
    EndedSyllable {
        /// 标记这个音节是否属于背景人声
        was_background: bool,
    },
}

#[derive(Debug, Deserialize, Default)]
struct Metadata {
    #[serde(rename = "meta", alias = "amll:meta", default)]
    metas: Vec<MetaTag>,
    #[serde(rename = "agent", alias = "ttm:agent", default)]
    agents: Vec<Agent>,
    #[serde(rename = "iTunesMetadata", default)]
    itunes_metadata: Option<ItunesMetadata>,
    #[serde(flatten)]
    other_metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Default)]
struct MetaTag {
    #[serde(rename = "@key")]
    key: String,
    #[serde(rename = "@value")]
    value: String,
}

#[derive(Debug, Deserialize, Default)]
struct Agent {
    #[serde(rename = "@xml:id")]
    id: String,
    #[serde(rename = "@type", default)]
    agent_type: String,
    #[serde(rename = "name", alias = "ttm:name", default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ItunesMetadata {
    #[serde(default)]
    songwriters: Songwriters,
    #[serde(default)]
    translations: Vec<ItunesTranslation>,
}

#[derive(Debug, Deserialize, Default)]
struct Songwriters {
    #[serde(rename = "songwriter", default)]
    list: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ItunesTranslation {
    #[serde(rename = "@xml:lang")]
    lang: Option<String>,
    #[serde(rename = "text", default)]
    texts: Vec<ItunesTranslationText>,
}

#[derive(Debug, Deserialize, Default)]
struct ItunesTranslationText {
    #[serde(rename = "@for")]
    key: String,
    #[serde(rename = "$text")]
    text: String,
}

// =================================================================================
// 3. 公共 API
// =================================================================================

/// 解析 TTML 格式的歌词文件。
///
/// # 参数
///
/// * `content` - TTML 格式的歌词文件内容字符串。
/// * `default_languages` - 当 TTML 文件中未指定语言时，使用的默认语言配置。
///
/// # 返回
///
/// * `Ok(ParsedSourceData)` - 成功解析后，返回包含歌词行、元数据等信息的统一数据结构。
/// * `Err(ConvertError)` - 解析失败时，返回具体的错误信息。
pub fn parse_ttml(
    content: &str,
    options: &TtmlParsingOptions,
) -> Result<ParsedSourceData, ConvertError> {
    // 预扫描以确定是否存在带时间的span，辅助判断计时模式
    let has_timed_span_tags = content.contains("<span") && content.contains("begin=");

    let mut reader = Reader::from_str(content);
    let config = reader.config_mut();
    // 显式配置读取器不自动 trim 文本前后的空白。
    config.trim_text(false);
    config.expand_empty_elements = true;

    // 初始化最终要返回的数据容器
    let estimated_lines = content.matches("<p").count();
    let mut lines: Vec<LyricLine> = Vec::with_capacity(estimated_lines);
    let mut raw_metadata: HashMap<String, Vec<String>> = HashMap::new();
    let mut warnings: Vec<String> = Vec::new();

    // 初始化解析状态机
    let mut state = TtmlParserState {
        default_main_lang: options.default_languages.main.clone(),
        default_translation_lang: options.default_languages.translation.clone(),
        default_romanization_lang: options.default_languages.romanization.clone(),
        ..Default::default()
    };
    let mut buf = Vec::new();

    loop {
        let event_start_pos = reader.buffer_position();

        if state.format_detection == FormatDetection::Undetermined {
            state.total_nodes_processed += 1;
            if state.whitespace_nodes_with_newline > 5 {
                state.format_detection = FormatDetection::IsFormatted;
            }
            // 避免在超大文件中扫描过久
            else if state.total_nodes_processed > 5000 {
                state.format_detection = FormatDetection::NotFormatted;
            }
        }

        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event,
            Err(e) => {
                // 格式错误，尝试继续，但状态机状态可能已经错乱
                // TODO: 加入数据恢复逻辑
                match e {
                    QuickXmlError::IllFormed(ill_formed_err) => {
                        warnings.push(format!(
                            "TTML 格式错误，位置 {}: {}。",
                            reader.error_position(),
                            ill_formed_err
                        ));
                        continue;
                    }
                    // 无法恢复的 IO 错误等
                    _ => {
                        error!(
                            "TTML 解析错误，位置 {}: {}。无法继续解析",
                            reader.error_position(),
                            e
                        );
                        return Err(ConvertError::Xml(e));
                    }
                }
            }
        };

        if let Event::Text(e) = &event
            && state.format_detection == FormatDetection::Undetermined
            && let Ok(text) = e.decode()
            && text.contains('\n')
            && text.trim().is_empty()
        {
            state.whitespace_nodes_with_newline += 1;
        }

        if let Event::Start(e) = &event
            && e.local_name().as_ref() == TAG_METADATA
        {
            reader.read_to_end(e.name())?;
            let end_pos = reader.buffer_position();
            let metadata_slice = &content[event_start_pos as usize..end_pos as usize];

            match from_str::<Metadata>(metadata_slice) {
                Ok(metadata_struct) => {
                    process_deserialized_metadata(metadata_struct, &mut state, &mut raw_metadata);
                }
                Err(de_error) => {
                    warnings.push(format!("解析 metadata 标签失败: {de_error}"));
                }
            }
            buf.clear();
            continue;
        }

        if state.body_state.in_p {
            if let Event::Eof = event {
                break;
            }
            handle_p_event(&event, &mut state, &reader, &mut lines, &mut warnings)?;
        } else {
            if let Event::Eof = event {
                break;
            }
            handle_global_event(
                &event,
                &mut state,
                &reader,
                &mut raw_metadata,
                &mut warnings,
                has_timed_span_tags,
                options,
            )?;
        }

        buf.clear();
    }

    Ok(ParsedSourceData {
        lines,
        raw_metadata,
        source_format: LyricFormat::Ttml,
        source_filename: None,
        is_line_timed_source: state.is_line_timing_mode,
        warnings,
        raw_ttml_from_input: Some(content.to_string()),
        detected_formatted_ttml_input: Some(state.format_detection == FormatDetection::IsFormatted),
        ..Default::default()
    })
}

// =================================================================================
// 4. 核心事件分发器
// =================================================================================

/// 处理全局事件（在 `<p>` 或 `<metadata>` 之外的事件）。
/// 主要负责识别文档的根元素、body、div 和 p 的开始，并相应地更新状态。
fn handle_global_event<'a>(
    event: &Event<'a>,
    state: &mut TtmlParserState,
    reader: &Reader<&[u8]>,
    raw_metadata: &mut HashMap<String, Vec<String>>,
    warnings: &mut Vec<String>,
    has_timed_span_tags: bool,
    options: &TtmlParsingOptions,
) -> Result<(), ConvertError> {
    match event {
        Event::Start(e) => match e.local_name().as_ref() {
            TAG_TT => process_tt_start(
                e,
                state,
                raw_metadata,
                reader,
                has_timed_span_tags,
                warnings,
                options,
            )?,

            TAG_BODY => state.body_state.in_body = true,
            TAG_DIV if state.body_state.in_body => {
                state.body_state.in_div = true;
                // 获取 song-part
                state.body_state.current_div_song_part = e
                    .try_get_attribute(ATTR_ITUNES_SONG_PART)?
                    .map(|attr| -> Result<String, ConvertError> {
                        Ok(attr
                            .decode_and_unescape_value(reader.decoder())?
                            .into_owned())
                    })
                    .transpose()?;
            }
            TAG_P if state.body_state.in_body => {
                state.body_state.in_p = true;

                // 获取 p 标签的各个属性
                let start_ms = get_time_attribute(e, reader, &[ATTR_BEGIN])?.unwrap_or(0);
                let end_ms = get_time_attribute(e, reader, &[ATTR_END])?.unwrap_or(0);

                let agent_id = get_string_attribute(e, reader, &[ATTR_AGENT, ATTR_AGENT_ALIAS])?;

                let agent_name = agent_id
                    .and_then(|id| state.metadata_state.agent_id_to_name_map.get(&id).cloned());

                let song_part = get_string_attribute(e, reader, &[ATTR_ITUNES_SONG_PART])?
                    .or(state.body_state.current_div_song_part.clone());
                let itunes_key = get_string_attribute(e, reader, &[ATTR_ITUNES_KEY])?;

                // 创建 p 元素数据容器
                state.body_state.current_p_element_data = Some(CurrentPElementData {
                    start_ms,
                    end_ms,
                    agent: agent_name,
                    song_part,
                    itunes_key,
                    ..Default::default()
                });

                // 重置 p 内部的状态
                state.text_buffer.clear();
                state.body_state.span_stack.clear();
            }
            _ => {}
        },
        Event::End(e) => match e.local_name().as_ref() {
            TAG_DIV if state.body_state.in_div => {
                state.body_state.in_div = false;
                state.body_state.current_div_song_part = None; // 离开 div 时清除
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

/// 处理在 `<p>` 标签内部的事件。
fn handle_p_event<'a>(
    event: &Event<'a>,
    state: &mut TtmlParserState,
    reader: &Reader<&[u8]>,
    lines: &mut Vec<LyricLine>,
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    match event {
        Event::Start(e) if e.local_name().as_ref() == TAG_SPAN => {
            process_span_start(e, state, reader)?;
        }
        Event::Text(e) => process_text_event(e, state)?,
        Event::GeneralRef(e) => {
            let entity_name = str::from_utf8(e.as_ref())
                .map_err(|err| ConvertError::Internal(format!("无法将实体名解码为UTF-8: {err}")))?;
            let decoded_char = match entity_name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                _ => {
                    warnings.push(format!("忽略了未知的XML实体 '&{entity_name};'"));
                    '\0'
                }
            };

            if decoded_char != '\0'
                && let Some(p_data) = state.body_state.current_p_element_data.as_mut()
            {
                if !state.body_state.span_stack.is_empty() {
                    state.text_buffer.push(decoded_char);
                } else {
                    p_data.line_text_accumulator.push(decoded_char);
                }
            }
        }

        Event::End(e) => {
            match e.local_name().as_ref() {
                TAG_BR => {
                    warnings.push(format!(
                        "在 <p> ({}ms-{}ms) 中发现并忽略了一个 <br/> 标签。",
                        state
                            .body_state
                            .current_p_element_data
                            .as_ref()
                            .map_or(0, |d| d.start_ms),
                        state
                            .body_state
                            .current_p_element_data
                            .as_ref()
                            .map_or(0, |d| d.end_ms)
                    ));
                }
                TAG_P => {
                    // 当 </p> 出现时，意味着一行歌词的数据已经全部收集完毕。
                    // 调用 finalize_p_element 来处理和整合这些数据。
                    if let Some(mut p_data) = state.body_state.current_p_element_data.take() {
                        // 回填来自 <iTunesMetadata> 的翻译
                        if let Some(key) = &p_data.itunes_key
                            && let Some((text, lang)) =
                                state.metadata_state.translation_map.get(key)
                        {
                            // 避免重复添加
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
                        finalize_p_element(p_data, lines, state, warnings);
                    }
                    // 重置 p 内部的状态
                    state.body_state.in_p = false;
                    state.body_state.span_stack.clear();
                    state.body_state.last_syllable_info = LastSyllableInfo::None;
                }
                TAG_SPAN => {
                    process_span_end(state, warnings)?;
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

// =================================================================================
// 5. XML 元素与事件处理器
// =================================================================================

/// 处理 `<tt>` 标签的开始事件，这是文档的根元素。
/// 主要任务是确定计时模式（逐行 vs 逐字）和文档的默认语言。
fn process_tt_start(
    e: &BytesStart,
    state: &mut TtmlParserState,
    raw_metadata: &mut HashMap<String, Vec<String>>,
    reader: &Reader<&[u8]>,
    has_timed_span_tags: bool,
    warnings: &mut Vec<String>,
    options: &TtmlParsingOptions,
) -> Result<(), ConvertError> {
    if let Some(forced_mode) = options.force_timing_mode {
        state.is_line_timing_mode = forced_mode == TtmlTimingMode::Line;
    } else {
        let timing_attr = e.try_get_attribute(ATTR_ITUNES_TIMING)?;
        if let Some(attr) = timing_attr {
            if attr.value.as_ref() == b"line" {
                state.is_line_timing_mode = true;
            }
        } else if !has_timed_span_tags {
            state.is_line_timing_mode = true;
            state.detected_line_mode = true;
            warnings.push(
                "未找到带时间戳的 <span> 标签且未指定 itunes:timing 模式，切换到逐行歌词模式。"
                    .to_string(),
            );
        }
    }

    // 获取 xml:lang 属性
    if let Some(attr) = e.try_get_attribute(ATTR_XML_LANG)? {
        let lang_val = attr.decode_and_unescape_value(reader.decoder())?;
        if !lang_val.is_empty() {
            let lang_val_owned = lang_val.into_owned();
            raw_metadata
                .entry("xml:lang_root".to_string())
                .or_default()
                .push(lang_val_owned.clone());
            if state.default_main_lang.is_none() {
                state.default_main_lang = Some(lang_val_owned);
            }
        }
    }

    Ok(())
}

/// 处理 `<span>` 标签的开始事件。
/// 这是解析器中最复杂的部分之一，需要确定 span 的角色、语言和时间信息。
fn process_span_start(
    e: &BytesStart,
    state: &mut TtmlParserState,
    reader: &Reader<&[u8]>,
) -> Result<(), ConvertError> {
    // 进入新的 span 前，清空文本缓冲区
    state.text_buffer.clear();

    // 获取 span 的各个属性
    let role = get_attribute_with_aliases(e, reader, &[ATTR_ROLE, ATTR_ROLE_ALIAS], |s| {
        Ok(match s.as_bytes() {
            ROLE_TRANSLATION => SpanRole::Translation,
            ROLE_ROMANIZATION => SpanRole::Romanization,
            ROLE_BACKGROUND => SpanRole::Background,
            _ => SpanRole::Generic,
        })
    })?
    .unwrap_or(SpanRole::Generic);

    let lang = get_string_attribute(e, reader, &[ATTR_XML_LANG])?;
    let scheme = get_string_attribute(e, reader, &[ATTR_XML_SCHEME])?;
    let start_ms = get_time_attribute(e, reader, &[ATTR_BEGIN])?;
    let end_ms = get_time_attribute(e, reader, &[ATTR_END])?;

    // 将解析出的上下文压入堆栈，以支持嵌套 span
    state.body_state.span_stack.push(SpanContext {
        role,
        lang,
        scheme,
        start_ms,
        end_ms,
    });

    // 如果是背景人声容器的开始，则初始化背景数据累加器
    if role == SpanRole::Background
        && let Some(p_data) = state.body_state.current_p_element_data.as_mut()
        && p_data.background_section_accumulator.is_none()
    {
        p_data.background_section_accumulator = Some(BackgroundSectionData {
            start_ms: start_ms.unwrap_or(0),
            end_ms: end_ms.unwrap_or(0),
            ..Default::default()
        });
    }
    Ok(())
}

/// 处理文本事件。
/// 这个函数的核心逻辑是区分 "音节间的空格" 和 "音节内的文本"。
fn process_text_event(e_text: &BytesText, state: &mut TtmlParserState) -> Result<(), ConvertError> {
    let text_slice = e_text.decode()?;

    if !state.body_state.in_p {
        return Ok(()); // 不在 <p> 标签内，忽略任何文本
    }

    // 如果上一个事件是一个结束的音节 (</span>)，并且当前文本是纯空格，
    // 那么这个空格应该附加到上一个音节上。
    if let LastSyllableInfo::EndedSyllable { was_background } = state.body_state.last_syllable_info
        && !text_slice.is_empty()
        && text_slice.chars().all(char::is_whitespace)
    {
        let has_space = state.format_detection == FormatDetection::NotFormatted
            || (!text_slice.contains('\n') && !text_slice.contains('\r'));

        if has_space && let Some(p_data) = state.body_state.current_p_element_data.as_mut() {
            // 根据上一个音节是否是背景人声，找到正确的音节列表
            let target_syllables = if was_background {
                p_data
                    .background_section_accumulator
                    .as_mut()
                    .map(|bs| &mut bs.syllables)
            } else {
                Some(&mut p_data.syllables_accumulator)
            };

            // 更新最后一个音节的 `ends_with_space` 标志
            if let Some(last_syl) = target_syllables.and_then(|s| s.last_mut())
                && !last_syl.ends_with_space
            {
                last_syl.ends_with_space = true;
            }
        }
        // 消费掉这个空格，并重置状态，然后直接返回
        state.body_state.last_syllable_info = LastSyllableInfo::None;
        return Ok(());
    }

    // 如果不是音节间空格，则处理常规文本
    let trimmed_text = text_slice.trim();
    if trimmed_text.is_empty() {
        // 如果trim后为空（意味着它不是音节间空格，只是普通的空白节点），则忽略
        return Ok(());
    }

    // 任何非音节间空格的文本出现后，重置 `last_syllable_info`
    state.body_state.last_syllable_info = LastSyllableInfo::None;

    // 累加到缓冲区
    if !state.body_state.span_stack.is_empty() {
        // 如果在 span 内，文本属于这个 span
        state.text_buffer.push_str(&text_slice);
    } else if let Some(p_data) = state.body_state.current_p_element_data.as_mut() {
        // 如果在 p 内但在任何 span 外，文本直接属于 p
        p_data.line_text_accumulator.push_str(&text_slice);
    }

    Ok(())
}

/// 处理 `</span>` 结束事件的分发器。
fn process_span_end(
    state: &mut TtmlParserState,
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    // 重置，因为 span 已经结束
    state.body_state.last_syllable_info = LastSyllableInfo::None;

    // 从堆栈中弹出刚刚结束的 span 的上下文
    if let Some(ended_span_ctx) = state.body_state.span_stack.pop() {
        // 获取并清空缓冲区中的文本
        let raw_text_from_buffer = std::mem::take(&mut state.text_buffer);

        // 根据 span 的角色分发给不同的处理器
        match ended_span_ctx.role {
            SpanRole::Generic => {
                handle_generic_span_end(state, &ended_span_ctx, &raw_text_from_buffer, warnings)?
            }
            SpanRole::Translation | SpanRole::Romanization => {
                handle_auxiliary_span_end(state, &ended_span_ctx, &raw_text_from_buffer)?
            }
            SpanRole::Background => {
                handle_background_span_end(state, &ended_span_ctx, &raw_text_from_buffer, warnings)?
            }
        }
    }
    Ok(())
}

/// 处理普通音节 `<span>` 结束的逻辑。
fn handle_generic_span_end(
    state: &mut TtmlParserState,
    ctx: &SpanContext,
    text: &str,
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    // 在逐行模式下，所有 span 的文本都简单地累加到行文本中
    if state.is_line_timing_mode {
        if let Some(p_data) = state.body_state.current_p_element_data.as_mut() {
            p_data.line_text_accumulator.push_str(text);
        }
        return Ok(());
    }

    if let (Some(start_ms), Some(end_ms)) = (ctx.start_ms, ctx.end_ms) {
        // 如果 span 内有任何内容（包括纯空格），就处理它
        if !text.is_empty() {
            if start_ms > end_ms {
                warnings.push(format!(
                    "音节 '{}' 的时间戳无效 (start_ms {} > end_ms {}), 但仍会创建音节。",
                    text.escape_debug(),
                    start_ms,
                    end_ms
                ));
            }

            let p_data = state
                .body_state
                .current_p_element_data
                .as_mut()
                .ok_or_else(|| {
                    ConvertError::Internal("在处理 span 时丢失了 p_data 上下文".to_string())
                })?;
            let was_within_bg = state
                .body_state
                .span_stack
                .iter()
                .any(|s| s.role == SpanRole::Background);
            let trimmed_text = text.trim();

            // 根据内容创建不同类型的音节
            let syllable = if !trimmed_text.is_empty() {
                state.text_processing_buffer.clear();
                if was_within_bg {
                    clean_parentheses_from_bg_text_into(
                        trimmed_text,
                        &mut state.text_processing_buffer,
                    );
                } else {
                    normalize_text_whitespace_into(trimmed_text, &mut state.text_processing_buffer);
                }

                LyricSyllable {
                    text: state.text_processing_buffer.clone(),
                    start_ms,
                    end_ms: end_ms.max(start_ms),
                    duration_ms: Some(end_ms.saturating_sub(start_ms)),
                    ends_with_space: text.ends_with(char::is_whitespace),
                }
            } else {
                // Case B: 这是一个只包含空格的音节
                LyricSyllable {
                    text: " ".to_string(), // 规范化为单个空格
                    start_ms,
                    end_ms: end_ms.max(start_ms),
                    duration_ms: Some(end_ms.saturating_sub(start_ms)),
                    ends_with_space: false,
                }
            };

            // 将创建好的音节添加到正确的列表中
            let target_syllables = if was_within_bg {
                p_data
                    .background_section_accumulator
                    .as_mut()
                    .map(|bs| &mut bs.syllables)
            } else {
                Some(&mut p_data.syllables_accumulator)
            };

            if let Some(syllables) = target_syllables {
                syllables.push(syllable);
                state.body_state.last_syllable_info = LastSyllableInfo::EndedSyllable {
                    was_background: was_within_bg,
                };
            }
        }
        // 如果 text.is_empty() (例如 <span ...></span>), 则自然忽略
    } else if !text.trim().is_empty() {
        // span 内有文本但没有时间信息，发出警告
        warnings.push(format!(
            "逐字模式下，span缺少时间信息，文本 '{}' 被忽略。",
            text.trim().escape_debug()
        ));
    }

    Ok(())
}

fn normalize_text_whitespace_into(input: &str, output: &mut String) {
    output.clear();
    let mut first = true;
    for word in input.split_whitespace() {
        if !first {
            output.push(' ');
        }
        output.push_str(word);
        first = false;
    }
}

/// 处理翻译和罗马音 `<span>` 结束的逻辑。
fn handle_auxiliary_span_end(
    state: &mut TtmlParserState,
    ctx: &SpanContext,
    text: &str,
) -> Result<(), ConvertError> {
    normalize_text_whitespace_into(text, &mut state.text_processing_buffer);
    if state.text_processing_buffer.is_empty() {
        return Ok(());
    }

    let p_data = state
        .body_state
        .current_p_element_data
        .as_mut()
        .ok_or_else(|| {
            ConvertError::Internal("在处理辅助 span 时丢失了 p_data 上下文".to_string())
        })?;

    // 检查是否在背景人声容器内
    let was_within_bg = state
        .body_state
        .span_stack
        .iter()
        .any(|s| s.role == SpanRole::Background);

    // 确定语言，优先使用 span 自身的 xml:lang，否则使用全局默认值
    let lang_to_use = ctx.lang.clone().or_else(|| match ctx.role {
        SpanRole::Translation => state.default_translation_lang.clone(),
        SpanRole::Romanization => state.default_romanization_lang.clone(),
        _ => None,
    });

    match ctx.role {
        SpanRole::Translation => {
            let entry = TranslationEntry {
                text: state.text_processing_buffer.clone(),
                lang: lang_to_use,
            };
            // 添加到正确的累加器
            if was_within_bg {
                if let Some(bg_section) = p_data.background_section_accumulator.as_mut() {
                    bg_section.translations.push(entry);
                }
            } else {
                p_data.translations_accumulator.push(entry);
            }
        }
        SpanRole::Romanization => {
            let entry = RomanizationEntry {
                text: state.text_processing_buffer.clone(),
                lang: lang_to_use,
                scheme: ctx.scheme.clone(),
            };
            if was_within_bg {
                if let Some(bg_section) = p_data.background_section_accumulator.as_mut() {
                    bg_section.romanizations.push(entry);
                }
            } else {
                p_data.romanizations_accumulator.push(entry);
            }
        }
        _ => {} // 不应该发生
    }
    Ok(())
}

/// 处理背景人声容器 `<span>` 结束的逻辑。
fn handle_background_span_end(
    state: &mut TtmlParserState,
    ctx: &SpanContext,
    text: &str, // 背景容器直接包含的文本
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    let p_data = state
        .body_state
        .current_p_element_data
        .as_mut()
        .ok_or_else(|| {
            ConvertError::Internal("在处理背景 span 时丢失了 p_data 上下文".to_string())
        })?;

    // 如果背景容器本身没有时间戳，但内部有带时间戳的音节，
    // 则根据内部音节的时间范围来推断容器的时间范围。
    if let Some(bg_acc) = p_data.background_section_accumulator.as_mut()
        && (ctx.start_ms.is_none() || ctx.end_ms.is_none())
        && !bg_acc.syllables.is_empty()
    {
        bg_acc.start_ms = bg_acc
            .syllables
            .iter()
            .map(|s| s.start_ms)
            .min()
            .unwrap_or(bg_acc.start_ms);
        bg_acc.end_ms = bg_acc
            .syllables
            .iter()
            .map(|s| s.end_ms)
            .max()
            .unwrap_or(bg_acc.end_ms);
    }

    // 处理不规范的情况：背景容器直接包含文本，而不是通过嵌套的 span。
    let trimmed_text = text.trim();
    if !trimmed_text.is_empty() {
        warn!(
            "<span ttm:role='x-bg'> 直接包含文本 '{}'。",
            trimmed_text.escape_debug()
        );
        if let (Some(start_ms), Some(end_ms)) = (ctx.start_ms, ctx.end_ms) {
            if let Some(bg_acc) = p_data.background_section_accumulator.as_mut() {
                // 只有在背景容器内部没有其他音节时，才将此直接文本视为一个音节
                if bg_acc.syllables.is_empty() {
                    normalize_text_whitespace_into(trimmed_text, &mut state.text_processing_buffer);
                    bg_acc.syllables.push(LyricSyllable {
                        text: state.text_processing_buffer.clone(),
                        start_ms,
                        end_ms: end_ms.max(start_ms),
                        duration_ms: Some(end_ms.saturating_sub(start_ms)),
                        ends_with_space: !text.is_empty() && text.ends_with(char::is_whitespace),
                    });
                    state.body_state.last_syllable_info = LastSyllableInfo::EndedSyllable {
                        was_background: true,
                    };
                } else {
                    warnings.push(format!("<span ttm:role='x-bg'> 直接包含文本 '{}'，但其内部已有音节，此直接文本被忽略。", trimmed_text.escape_debug()));
                }
            }
        } else {
            warnings.push(format!(
                "<span ttm:role='x-bg'> 直接包含文本 '{}'，但缺少时间信息，忽略。",
                trimmed_text.escape_debug()
            ));
        }
    }
    Ok(())
}

// =================================================================================
// 6. 数据终结逻辑
// =================================================================================

/// 在 `</p>` 结束时，终结并处理一个 `LyricLine`。
/// 这个函数负责将 `CurrentPElementData` 中的所有累积数据，
/// 组合成一个完整的 `LyricLine` 对象，并添加到最终结果中。
fn finalize_p_element(
    p_data: CurrentPElementData,
    lines: &mut Vec<LyricLine>,
    state: &mut TtmlParserState,
    warnings: &mut Vec<String>,
) {
    let CurrentPElementData {
        start_ms,
        end_ms,
        agent,
        song_part,
        line_text_accumulator,
        syllables_accumulator,
        translations_accumulator,
        romanizations_accumulator,
        background_section_accumulator,
        itunes_key,
    } = p_data;

    // 创建一个初步的 LyricLine
    let mut final_line = LyricLine {
        start_ms,
        end_ms,
        itunes_key,
        agent: agent.or_else(|| Some("v1".to_string())), // 默认 agent 为 v1
        song_part,
        translations: translations_accumulator,
        romanizations: romanizations_accumulator,
        ..Default::default()
    };

    // 根据计时模式，调用不同的处理逻辑
    if state.is_line_timing_mode {
        finalize_p_for_line_mode(
            &mut final_line,
            &line_text_accumulator,
            &syllables_accumulator,
            warnings,
            &mut state.text_processing_buffer,
        );
    } else {
        finalize_p_for_word_mode(
            &mut final_line,
            syllables_accumulator,
            &line_text_accumulator,
            warnings,
            &mut state.text_processing_buffer,
        );
    }

    // 处理累积的背景人声部分
    if let Some(bg_data) = background_section_accumulator
        && (!bg_data.syllables.is_empty()
            || !bg_data.translations.is_empty()
            || !bg_data.romanizations.is_empty())
    {
        final_line.background_section = Some(BackgroundSection {
            start_ms: bg_data.start_ms,
            end_ms: bg_data.end_ms,
            syllables: bg_data.syllables,
            translations: bg_data.translations,
            romanizations: bg_data.romanizations,
        });
    }

    if let Some(last_syl) = final_line.main_syllables.last_mut() {
        last_syl.ends_with_space = false;
    }
    if let Some(bg_section) = final_line.background_section.as_mut()
        && let Some(last_bg_syl) = bg_section.syllables.last_mut()
    {
        last_bg_syl.ends_with_space = false;
    }

    // 如果行有文本但没有音节，创建一个代表整行的音节
    if final_line.main_syllables.is_empty()
        && let Some(line_text) = final_line.line_text.as_ref().filter(|s| !s.is_empty())
        && final_line.end_ms > final_line.start_ms
    {
        final_line.main_syllables.push(LyricSyllable {
            text: line_text.clone(),
            start_ms: final_line.start_ms,
            end_ms: final_line.end_ms,
            duration_ms: Some(final_line.end_ms.saturating_sub(final_line.start_ms)),
            ends_with_space: false,
        });
    }

    if final_line.main_syllables.is_empty()
        && final_line.line_text.as_deref().is_none_or(str::is_empty)
        && final_line.translations.is_empty()
        && final_line.romanizations.is_empty()
        && final_line.background_section.is_none()
        && final_line.end_ms <= final_line.start_ms
    {
        return;
    }

    lines.push(final_line);
}

/// 处理逐行模式下 `<p>` 元素结束的逻辑。
fn finalize_p_for_line_mode(
    final_line: &mut LyricLine,
    line_text_accumulator: &str,
    syllables_accumulator: &[LyricSyllable],
    warnings: &mut Vec<String>,
    text_processing_buffer: &mut String,
) {
    let mut line_text_content = line_text_accumulator.to_string();

    // 兼容处理：如果 p 内没有直接文本，但有带文本的 span，
    // 则将这些 span 的文本拼接起来作为行文本。
    if line_text_content.trim().is_empty() && !syllables_accumulator.is_empty() {
        line_text_content = syllables_accumulator
            .iter()
            .map(|s| {
                if s.ends_with_space {
                    format!("{} ", s.text)
                } else {
                    s.text.clone()
                }
            })
            .collect::<String>();
        warnings.push(format!(
            "逐行段落 ({}ms-{}ms) 的文本来自其内部的逐字结构。",
            final_line.start_ms, final_line.end_ms
        ));
    }

    normalize_text_whitespace_into(&line_text_content, text_processing_buffer);
    final_line.line_text = Some(text_processing_buffer.clone());

    // 在逐行模式下，音节的时间戳被忽略，记录一个警告。
    if !syllables_accumulator.is_empty() {
        warnings.push(format!(
            "在逐行歌词的段落 ({}ms-{}ms) 中检测到并忽略了 {} 个逐字音节的时间戳。",
            final_line.start_ms,
            final_line.end_ms,
            syllables_accumulator.len()
        ));
    }
}

/// 处理逐字模式下 `<p>` 元素结束的逻辑。
fn finalize_p_for_word_mode(
    final_line: &mut LyricLine,
    syllables_accumulator: Vec<LyricSyllable>,
    line_text_accumulator: &str,
    warnings: &mut Vec<String>,
    text_processing_buffer: &mut String,
) {
    final_line.main_syllables = syllables_accumulator;

    // 处理那些在 `<p>` 标签内但没有被 `<span>` 包裹的文本。
    normalize_text_whitespace_into(line_text_accumulator, text_processing_buffer);
    if !text_processing_buffer.is_empty() {
        if final_line.main_syllables.is_empty() {
            // 如果行内没有任何音节，则将这些文本视为一个覆盖整行时间的音节。
            let syl_start = final_line.start_ms;
            let syl_end = final_line.end_ms;
            if syl_start > syl_end {
                warnings.push(format!("为 <p> 标签内的直接文本 '{}' 创建音节时，时间戳无效 (start_ms {} > end_ms {}).", text_processing_buffer.escape_debug(), syl_start, syl_end));
            }
            final_line.main_syllables.push(LyricSyllable {
                text: text_processing_buffer.clone(),
                start_ms: syl_start,
                end_ms: syl_end.max(syl_start),
                duration_ms: Some(syl_end.saturating_sub(syl_start)),
                ends_with_space: false,
            });
        } else {
            // 如果行内已有音节，这些未被包裹的文本通常是无意义的，记录警告并忽略。
            warnings.push(format!(
                "段落 ({}ms-{}ms) 包含未被span包裹的文本: '{}'。此文本被忽略。",
                final_line.start_ms,
                final_line.end_ms,
                text_processing_buffer.escape_debug()
            ));
        }
    }

    // 根据音节列表，重新组装整行的文本 `line_text`。
    if final_line.line_text.is_none() && !final_line.main_syllables.is_empty() {
        let assembled_line_text = final_line
            .main_syllables
            .iter()
            .map(|s| {
                if s.ends_with_space {
                    format!("{} ", s.text)
                } else {
                    s.text.clone()
                }
            })
            .collect::<String>();
        final_line.line_text = Some(assembled_line_text.trim_end().to_string());
    }
}

// =================================================================================
// 7. 工具函数
// =================================================================================

/// 解析 TTML 时间字符串到毫秒。
fn parse_ttml_time_to_ms(time_str: &str) -> Result<u64, ConvertError> {
    // 解析毫秒部分（.1, .12, .123）
    fn parse_decimal_ms_part(ms_str: &str, original_time_str: &str) -> Result<u64, ConvertError> {
        if ms_str.is_empty() || ms_str.len() > 3 || ms_str.chars().any(|c| !c.is_ascii_digit()) {
            return Err(ConvertError::InvalidTime(format!(
                "毫秒部分 '{ms_str}' 在时间戳 '{original_time_str}' 中无效或格式错误 (只支持最多3位数字)"
            )));
        }
        let val = ms_str.parse::<u64>().map_err(|e| {
            ConvertError::InvalidTime(format!(
                "无法解析时间戳 '{original_time_str}' 中的毫秒部分 '{ms_str}': {e}"
            ))
        })?;
        Ok(val * 10u64.pow(3 - ms_str.len() as u32))
    }

    // 解析 "SS.mmm" 或 "SS" 格式的字符串，返回秒和毫秒
    fn parse_seconds_and_decimal_ms_part(
        seconds_and_ms_str: &str,
        original_time_str: &str,
    ) -> Result<(u64, u64), ConvertError> {
        let mut dot_parts = seconds_and_ms_str.splitn(2, '.');
        let seconds_str = dot_parts.next().unwrap(); // 肯定有

        if seconds_str.is_empty() {
            // 例如 ".5s" 或 "MM:.5"
            return Err(ConvertError::InvalidTime(format!(
                "时间格式 '{original_time_str}' 的秒部分为空 (例如 '.mmm')"
            )));
        }

        let seconds = seconds_str.parse::<u64>().map_err(|e| {
            ConvertError::InvalidTime(format!(
                "在时间戳 '{original_time_str}' 中解析秒 '{seconds_str}' 失败: {e}"
            ))
        })?;

        let milliseconds = if let Some(ms_str) = dot_parts.next() {
            parse_decimal_ms_part(ms_str, original_time_str)?
        } else {
            0
        };

        Ok((seconds, milliseconds))
    }

    // 格式："12.345s"
    if let Some(stripped) = time_str.strip_suffix('s') {
        if stripped.is_empty() || stripped.starts_with('.') || stripped.ends_with('.') {
            return Err(ConvertError::InvalidTime(format!(
                "时间戳 '{time_str}' 包含无效的秒格式"
            )));
        }
        if stripped.starts_with('-') {
            return Err(ConvertError::InvalidTime(format!(
                "时间戳不能为负: '{time_str}'"
            )));
        }

        let (seconds, milliseconds) = parse_seconds_and_decimal_ms_part(stripped, time_str)?;

        Ok(seconds * 1000 + milliseconds)
    } else {
        // 格式："HH:MM:SS.mmm", "MM:SS.mmm", "SS.mmm"
        // 从后往前解析以简化逻辑
        let mut parts_iter = time_str.split(':').rev(); // 倒序迭代

        let mut total_ms: u64 = 0;

        // 解析最后一个部分 (SS.mmm 或 SS)
        let current_part_str = parts_iter.next().ok_or_else(|| {
            ConvertError::InvalidTime(format!("时间格式 '{time_str}' 无效或为空"))
        })?;

        if current_part_str.starts_with('-') {
            // 检查负数
            return Err(ConvertError::InvalidTime(format!(
                "时间戳不能为负: '{time_str}'"
            )));
        }

        let (seconds, milliseconds) =
            parse_seconds_and_decimal_ms_part(current_part_str, time_str)?;
        total_ms += seconds * 1000 + milliseconds;

        // 解析倒数第二个部分 (分钟 MM)
        if let Some(minutes_str) = parts_iter.next() {
            let minutes = minutes_str.parse::<u64>().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{time_str}' 中解析分钟 '{minutes_str}' 失败: {e}"
                ))
            })?;
            if minutes >= 60 {
                return Err(ConvertError::InvalidTime(format!(
                    "分钟值 '{minutes}' (应 < 60) 在时间戳 '{time_str}' 中无效"
                )));
            }
            total_ms += minutes * 60_000;
        }

        // 解析倒数第三个部分 (小时 HH)
        if let Some(hours_str) = parts_iter.next() {
            let hours = hours_str.parse::<u64>().map_err(|e| {
                ConvertError::InvalidTime(format!(
                    "在 '{time_str}' 中解析小时 '{hours_str}' 失败: {e}"
                ))
            })?;
            total_ms += hours * 3_600_000;
        }

        if parts_iter.next().is_some() {
            return Err(ConvertError::InvalidTime(format!(
                "时间格式 '{time_str}' 包含过多部分，格式无效。"
            )));
        }

        // 如果是单独的 "SS.mmm" 格式，秒数可以大于59。
        // 否则（HH:MM:SS 或 MM:SS），秒数必须小于60。
        let num_colon_parts = time_str.chars().filter(|&c| c == ':').count();
        if num_colon_parts > 0 && seconds >= 60 {
            return Err(ConvertError::InvalidTime(format!(
                "秒值 '{seconds}' (应 < 60) 在时间戳 '{time_str}' 中无效"
            )));
        }

        Ok(total_ms)
    }
}

/// 清理文本两端的括号（单个或成对）
fn clean_parentheses_from_bg_text_into(text: &str, output: &mut String) {
    output.clear();
    let trimmed = text
        .trim()
        .trim_start_matches(['(', '（'])
        .trim_end_matches([')', '）'])
        .trim();
    output.push_str(trimmed);
}

/// 辅助函数：处理从 serde 解析出的元数据
fn process_deserialized_metadata(
    metadata: Metadata,
    state: &mut TtmlParserState,
    raw_metadata: &mut HashMap<String, Vec<String>>,
) {
    for meta in metadata.metas {
        raw_metadata.entry(meta.key).or_default().push(meta.value);
    }

    for agent in metadata.agents {
        if let Some(name) = agent.name.as_ref().filter(|n| !n.is_empty()) {
            state
                .metadata_state
                .agent_id_to_name_map
                .insert(agent.id.clone(), name.clone());
        }

        let agent_display = match agent.name {
            Some(name) if !name.is_empty() => format!("{}={}", agent.id, name),
            _ => agent.id.clone(),
        };
        raw_metadata
            .entry("agent".to_string())
            .or_default()
            .push(agent_display);

        if !agent.agent_type.is_empty() {
            let type_key = format!("agent-type-{}", agent.id);
            raw_metadata
                .entry(type_key)
                .or_default()
                .push(agent.agent_type);
        }
    }

    if let Some(itunes) = metadata.itunes_metadata {
        if !itunes.songwriters.list.is_empty() {
            raw_metadata.insert("songwriters".to_string(), itunes.songwriters.list);
        }

        for trans in itunes.translations {
            for text_entry in trans.texts {
                state
                    .metadata_state
                    .translation_map
                    .insert(text_entry.key, (text_entry.text, trans.lang.clone()));
            }
        }
    }

    for (key, value) in metadata.other_metadata {
        state.text_processing_buffer.clear();
        normalize_text_whitespace_into(&value, &mut state.text_processing_buffer);
        if !state.text_processing_buffer.is_empty() {
            raw_metadata
                .entry(key)
                .or_default()
                .push(state.text_processing_buffer.clone());
        }
    }
}

/// 从给定的属性名列表中获取第一个找到的属性，并将其转换为目标类型。
///
/// # 参数
/// * `e` - `BytesStart` 事件，代表一个 XML 标签的开始。
/// * `reader` - XML 读取器，用于解码。
/// * `attr_names` - 一个字节切片数组，包含所有要尝试的属性名（包括别名）。
/// * `processor` - 一个闭包，接收解码后的字符串值，并返回 `Result<T, ConvertError>`。
///
/// # 返回
/// * `Result<Option<T>, ConvertError>` - 成功时返回一个包含转换后值的 Option，如果找不到任何属性则返回 `None`。
fn get_attribute_with_aliases<T, F>(
    e: &BytesStart,
    reader: &Reader<&[u8]>,
    attr_names: &[&[u8]],
    processor: F,
) -> Result<Option<T>, ConvertError>
where
    F: Fn(&str) -> Result<T, ConvertError>,
{
    let mut found_attr = None;
    for &name in attr_names {
        if let Some(attr) = e.try_get_attribute(name)? {
            found_attr = Some(attr);
            break;
        }
    }

    found_attr
        .map(|attr| {
            let decoded_value = attr.decode_and_unescape_value(reader.decoder())?;
            processor(&decoded_value)
        })
        .transpose()
}

/// 获取字符串类型的属性值。
fn get_string_attribute(
    e: &BytesStart,
    reader: &Reader<&[u8]>,
    attr_names: &[&[u8]],
) -> Result<Option<String>, ConvertError> {
    get_attribute_with_aliases(e, reader, attr_names, |s| Ok(s.to_owned()))
}

/// 获取并解析为毫秒的时间戳属性值。
fn get_time_attribute(
    e: &BytesStart,
    reader: &Reader<&[u8]>,
    attr_names: &[&[u8]],
) -> Result<Option<u64>, ConvertError> {
    get_attribute_with_aliases(e, reader, attr_names, parse_ttml_time_to_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConvertError;

    #[test]
    fn test_parse_ttml_time_to_ms() {
        assert_eq!(parse_ttml_time_to_ms("7.1s").unwrap(), 7100);
        assert_eq!(parse_ttml_time_to_ms("7.12s").unwrap(), 7120);
        assert_eq!(parse_ttml_time_to_ms("7.123s").unwrap(), 7123);
        assert_eq!(parse_ttml_time_to_ms("99999.123s").unwrap(), 99999123);
        assert_eq!(parse_ttml_time_to_ms("01:02:03.456").unwrap(), 3723456);
        assert_eq!(parse_ttml_time_to_ms("05:10.1").unwrap(), 310100);
        assert_eq!(parse_ttml_time_to_ms("05:10.12").unwrap(), 310120);
        assert_eq!(parse_ttml_time_to_ms("7.123").unwrap(), 7123);
        assert_eq!(parse_ttml_time_to_ms("7").unwrap(), 7000);
        assert_eq!(parse_ttml_time_to_ms("15.5s").unwrap(), 15500);
        assert_eq!(parse_ttml_time_to_ms("15s").unwrap(), 15000);

        assert_eq!(parse_ttml_time_to_ms("0").unwrap(), 0);
        assert_eq!(parse_ttml_time_to_ms("0.0s").unwrap(), 0);
        assert_eq!(parse_ttml_time_to_ms("00:00:00.000").unwrap(), 0);
        assert_eq!(parse_ttml_time_to_ms("99:59:59.999").unwrap(), 359999999);
        assert_eq!(parse_ttml_time_to_ms("60").unwrap(), 60000);
        assert_eq!(parse_ttml_time_to_ms("123.456").unwrap(), 123456);

        assert!(matches!(
            parse_ttml_time_to_ms("abc"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("1:2:3:4"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("01:60:00.000"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("01:00:60.000"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("-10s"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("-01:00:00.000"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("10.s"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms(".5s"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("s"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("10.1234s"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("10.abcs"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("10.1234"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("10.abc"),
            Err(ConvertError::InvalidTime(_))
        ));
        assert!(matches!(
            parse_ttml_time_to_ms("01:00:.000"),
            Err(ConvertError::InvalidTime(_))
        ));
    }

    #[test]
    fn test_normalize_text_whitespace() {
        let mut buffer = String::new();

        normalize_text_whitespace_into("  hello   world  ", &mut buffer);
        assert_eq!(buffer, "hello world");

        normalize_text_whitespace_into("\n\t  foo \r\n bar\t", &mut buffer);
        assert_eq!(buffer, "foo bar");

        normalize_text_whitespace_into("single", &mut buffer);
        assert_eq!(buffer, "single");

        normalize_text_whitespace_into("   ", &mut buffer);
        assert_eq!(buffer, "");

        normalize_text_whitespace_into("", &mut buffer);
        assert_eq!(buffer, "");
    }

    #[test]
    fn test_clean_parentheses_from_bg_text() {
        fn clean_parentheses_from_bg_text_into_owned(text: &str) -> String {
            let mut buf = String::new();
            clean_parentheses_from_bg_text_into(text, &mut buf);
            buf
        }

        assert_eq!(
            clean_parentheses_from_bg_text_into_owned("(hello)"),
            "hello"
        );
        assert_eq!(
            clean_parentheses_from_bg_text_into_owned("（hello）"),
            "hello"
        );
        assert_eq!(
            clean_parentheses_from_bg_text_into_owned(" ( hello world ) "),
            "hello world"
        );
        assert_eq!(
            clean_parentheses_from_bg_text_into_owned("(unmatched"),
            "unmatched"
        );
        assert_eq!(
            clean_parentheses_from_bg_text_into_owned("unmatched)"),
            "unmatched"
        );
        assert_eq!(
            clean_parentheses_from_bg_text_into_owned("no parentheses"),
            "no parentheses"
        );
    }
}
