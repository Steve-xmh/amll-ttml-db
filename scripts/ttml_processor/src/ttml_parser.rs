//! # TTML (Timed Text Markup Language) 解析器
//!
//! 该解析器设计上仅用于解析 Apple Music 和 AMLL 使用的 TTML 歌词文件，
//! 不建议用于解析通用的 TTML 字幕文件。

use std::{collections::HashMap, str};

use quick_xml::{
    Reader,
    errors::Error as QuickXmlError,
    events::{BytesStart, BytesText, Event},
};
use tracing::error;

use crate::types::{
    Agent, AgentStore, AgentType, AnnotatedTrack, ContentType, ConvertError, LyricFormat,
    LyricLine, LyricSyllable, LyricTrack, ParsedSourceData, TrackMetadataKey, TtmlParsingOptions,
    TtmlTimingMode, Word,
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

const TAG_AGENT: &[u8] = b"agent";
const TAG_AGENT_TTM: &[u8] = b"ttm:agent";
const TAG_NAME: &[u8] = b"name";
const TAG_NAME_TTM: &[u8] = b"ttm:name";
const TAG_META: &[u8] = b"meta";
const TAG_META_AMLL: &[u8] = b"amll:meta";
const TAG_ITUNES_METADATA: &[u8] = b"iTunesMetadata";
const TAG_SONGWRITER: &[u8] = b"songwriter";
const TAG_TRANSLATIONS: &[u8] = b"translations";
const TAG_TRANSLITERATIONS: &[u8] = b"transliterations";
const TAG_TRANSLATION: &[u8] = b"translation";
const TAG_TRANSLITERATION: &[u8] = b"transliteration";
const TAG_TEXT: &[u8] = b"text";

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
const ATTR_XML_ID: &[u8] = b"xml:id";
const ATTR_KEY: &[u8] = b"key";
const ATTR_VALUE: &[u8] = b"value";
const ATTR_FOR: &[u8] = b"for";

const ROLE_TRANSLATION: &[u8] = b"x-translation";
const ROLE_ROMANIZATION: &[u8] = b"x-roman";
const ROLE_BACKGROUND: &[u8] = b"x-bg";

// =================================================================================
// 2. 状态机和元数据结构体
// =================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum FormatDetection {
    #[default]
    Undetermined,
    IsFormatted,
    NotFormatted,
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

    in_metadata: bool,
    /// 存储 `<metadata>` 区域解析状态的结构体。
    metadata_state: MetadataParseState,
    /// 存储 `<body>` 和 `<p>` 区域解析状态的结构体。
    body_state: BodyParseState,

    /// 用于存储正在构建的 `AgentStore`
    agent_store: AgentStore,
    /// 用于为在 `<p>` 标签中直接发现的 `agent` 名称生成新ID的计数器
    agent_counter: u32,
    /// 用于存储已为直接名称生成的 `ID` 映射 (`name` -> `id`)
    agent_name_to_id_map: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy)]
enum AuxTrackType {
    Translation,
    Romanization,
}

#[derive(Debug, Default)]
enum MetadataContext {
    #[default]
    None, // 不在任何特殊元数据标签内
    InAgent {
        id: Option<String>,
    },
    InITunesMetadata,
    InSongwriter,
    InAuxiliaryContainer {
        // 代表 <translations> 或 <transliterations>
        aux_type: AuxTrackType,
    },
    InAuxiliaryEntry {
        // 代表 <translation> 或 <transliteration>
        aux_type: AuxTrackType,
        lang: Option<String>,
    },
    InAuxiliaryText {
        // 代表 <text>
        aux_type: AuxTrackType,
        lang: Option<String>,
        key: Option<String>,
    },
}

#[derive(Debug, Default, Clone)]
struct AuxiliaryTrackSet {
    translations: Vec<LyricTrack>,
    romanizations: Vec<LyricTrack>,
}

#[derive(Debug, Default, Clone)]
struct DetailedAuxiliaryTracks {
    main_tracks: AuxiliaryTrackSet,
    background_tracks: AuxiliaryTrackSet,
}

/// 存储 `<metadata>` 区域解析状态的结构体。
#[derive(Debug, Default)]
struct MetadataParseState {
    line_translation_map: HashMap<String, (LineTranslation, Option<String>)>,
    timed_track_map: HashMap<String, DetailedAuxiliaryTracks>,

    context: MetadataContext,
    current_main_syllables: Vec<LyricSyllable>,
    current_bg_syllables: Vec<LyricSyllable>,

    current_main_plain_text: String,
    current_bg_plain_text: String,

    span_stack: Vec<SpanContext>,
    text_buffer: String,
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
    /// 累积所有带注解的轨道数据
    tracks_accumulator: Vec<AnnotatedTrack>,
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

/// 用于存储从 `<head>` 中解析的逐行翻译。
#[derive(Debug, Default, Clone)]
struct LineTranslation {
    main: Option<String>,
    background: Option<String>,
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
    reader.config_mut().trim_text(false);
    reader.config_mut().expand_empty_elements = true;

    let mut lines: Vec<LyricLine> = Vec::with_capacity(content.matches("<p").count());
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
        if state.format_detection == FormatDetection::Undetermined {
            state.total_nodes_processed += 1;
            if state.whitespace_nodes_with_newline > 5 {
                state.format_detection = FormatDetection::IsFormatted;
            } else if state.total_nodes_processed > 5000 {
                state.format_detection = FormatDetection::NotFormatted;
            }
        }

        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event,
            Err(e) => {
                // 尝试抢救数据
                if let QuickXmlError::IllFormed(_) = e {
                    attempt_recovery_from_error(&mut state, &reader, &mut lines, &mut warnings, &e);
                    buf.clear();
                    continue;
                }

                // 无法恢复的 IO 错误等
                error!(
                    "TTML 解析错误，位置 {}: {}。无法继续解析",
                    reader.error_position(),
                    e
                );
                return Err(ConvertError::Xml(e));
            }
        };

        if let Event::Text(e) = &event
            && state.format_detection == FormatDetection::Undetermined
        {
            let bytes = e.as_ref();
            if bytes.contains(&b'\n') && bytes.iter().all(|&b| b.is_ascii_whitespace()) {
                state.whitespace_nodes_with_newline += 1;
            }
        }

        if let Event::Eof = event {
            break;
        }

        if state.in_metadata {
            handle_metadata_event(
                &event,
                &mut reader,
                &mut state,
                &mut raw_metadata,
                &mut warnings,
            )?;
        } else if state.body_state.in_p {
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
        agents: state.agent_store,
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

/// 处理 `<metadata>` 块内部的事件。
fn handle_metadata_event(
    event: &Event,
    reader: &mut Reader<&[u8]>,
    state: &mut TtmlParserState,
    raw_metadata: &mut HashMap<String, Vec<String>>,
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    let meta_state = &mut state.metadata_state;

    match event {
        Event::Start(e) => match e.name().as_ref() {
            TAG_AGENT | TAG_AGENT_TTM => {
                let id_opt = get_string_attribute(e, reader, &[ATTR_XML_ID])?;
                if let Some(id) = id_opt {
                    let type_str = get_string_attribute(e, reader, &[b"type"])?.unwrap_or_default();
                    let agent_type = match type_str.as_str() {
                        "person" => AgentType::Person,
                        "group" => AgentType::Group,
                        _ => AgentType::Other,
                    };

                    let agent = Agent {
                        id: id.clone(),
                        name: None,
                        agent_type,
                    };
                    state.agent_store.agents_by_id.insert(id.clone(), agent);
                    meta_state.context = MetadataContext::InAgent { id: Some(id) };
                } else {
                    warnings.push("发现一个没有 xml:id 的 <ttm:agent> 标签，已忽略。".to_string());
                }
            }
            TAG_NAME | TAG_NAME_TTM => {
                if let MetadataContext::InAgent { id: Some(agent_id) } = &meta_state.context {
                    let name = reader.read_text(e.name())?.into_owned();
                    if !name.trim().is_empty() {
                        // MODIFICATION: Update the name in the Agent struct within the store
                        if let Some(agent) = state.agent_store.agents_by_id.get_mut(agent_id) {
                            agent.name = Some(name.trim().to_string());
                        }
                    }
                }
            }
            TAG_META | TAG_META_AMLL => {
                let key_attr = get_string_attribute(e, reader, &[ATTR_KEY])?;
                let value_attr = get_string_attribute(e, reader, &[ATTR_VALUE])?;

                let text_content = reader.read_text(e.name())?;

                if let Some(key) = key_attr {
                    let value = value_attr.unwrap_or_else(|| text_content.into_owned());
                    if !key.is_empty() {
                        raw_metadata.entry(key).or_default().push(value);
                    }
                }
            }
            TAG_ITUNES_METADATA => meta_state.context = MetadataContext::InITunesMetadata,
            TAG_SONGWRITER => {
                if matches!(meta_state.context, MetadataContext::InITunesMetadata) {
                    meta_state.context = MetadataContext::InSongwriter;
                }
            }
            TAG_TRANSLATIONS => {
                if matches!(meta_state.context, MetadataContext::InITunesMetadata) {
                    meta_state.context = MetadataContext::InAuxiliaryContainer {
                        aux_type: AuxTrackType::Translation,
                    };
                }
            }
            TAG_TRANSLITERATIONS => {
                if matches!(meta_state.context, MetadataContext::InITunesMetadata) {
                    meta_state.context = MetadataContext::InAuxiliaryContainer {
                        aux_type: AuxTrackType::Romanization,
                    };
                }
            }
            TAG_TRANSLATION | TAG_TRANSLITERATION => {
                if let MetadataContext::InAuxiliaryContainer { aux_type } = meta_state.context {
                    let lang = get_string_attribute(e, reader, &[ATTR_XML_LANG])?;
                    meta_state.context = MetadataContext::InAuxiliaryEntry { aux_type, lang };
                }
            }
            TAG_TEXT => {
                if let MetadataContext::InAuxiliaryEntry { aux_type, lang } = &meta_state.context {
                    let key = get_string_attribute(e, reader, &[ATTR_FOR])?;
                    meta_state.context = MetadataContext::InAuxiliaryText {
                        aux_type: *aux_type,
                        lang: lang.clone(),
                        key,
                    };
                    meta_state.current_main_plain_text.clear();
                    meta_state.current_bg_plain_text.clear();
                    meta_state.current_main_syllables.clear();
                    meta_state.current_bg_syllables.clear();
                    meta_state.span_stack.clear();
                    meta_state.text_buffer.clear();
                }
            }
            TAG_SPAN => {
                if matches!(meta_state.context, MetadataContext::InAuxiliaryText { .. }) {
                    meta_state.text_buffer.clear();

                    let role = get_attribute_with_aliases(
                        e,
                        reader,
                        &[ATTR_ROLE, ATTR_ROLE_ALIAS],
                        |s| {
                            Ok(match s.as_bytes() {
                                ROLE_BACKGROUND => SpanRole::Background,
                                _ => SpanRole::Generic,
                            })
                        },
                    )?
                    .unwrap_or(SpanRole::Generic);

                    let start_ms = get_time_attribute(e, reader, &[ATTR_BEGIN], warnings)?;
                    let end_ms = get_time_attribute(e, reader, &[ATTR_END], warnings)?;

                    meta_state.span_stack.push(SpanContext {
                        role,
                        start_ms,
                        end_ms,
                        lang: None,
                        scheme: None,
                    });
                }
            }
            _ => {}
        },
        Event::Text(e) => {
            if !meta_state.span_stack.is_empty() {
                // 处理在 span 内部的文本
                meta_state.text_buffer.push_str(&e.xml_content()?);
            } else if matches!(meta_state.context, MetadataContext::InAuxiliaryText { .. }) {
                // 处理 `<text>` 标签直接子节点中的文本（即主翻译）
                meta_state
                    .current_main_plain_text
                    .push_str(&e.xml_content()?);
            } else if matches!(meta_state.context, MetadataContext::InSongwriter) {
                raw_metadata
                    .entry("songwriters".to_string())
                    .or_default()
                    .push(e.xml_content()?.into_owned());
            }
        }
        Event::End(e) => match e.name().as_ref() {
            TAG_METADATA => state.in_metadata = false,
            TAG_ITUNES_METADATA => meta_state.context = MetadataContext::None,
            TAG_SONGWRITER => meta_state.context = MetadataContext::InITunesMetadata,
            TAG_AGENT | TAG_AGENT_TTM => {
                meta_state.context = MetadataContext::None;
            }
            TAG_TRANSLATIONS | TAG_TRANSLITERATIONS => {
                meta_state.context = MetadataContext::InITunesMetadata;
            }
            TAG_TRANSLATION | TAG_TRANSLITERATION => {
                if let MetadataContext::InAuxiliaryEntry { aux_type, .. } = &meta_state.context {
                    meta_state.context = MetadataContext::InAuxiliaryContainer {
                        aux_type: *aux_type,
                    };
                }
            }
            TAG_SPAN => {
                if matches!(meta_state.context, MetadataContext::InAuxiliaryText { .. })
                    && let Some(ended_span_ctx) = meta_state.span_stack.pop()
                {
                    let raw_text = std::mem::take(&mut meta_state.text_buffer);

                    if let (Some(start_ms), Some(end_ms)) =
                        (ended_span_ctx.start_ms, ended_span_ctx.end_ms)
                    {
                        let is_within_background_container = meta_state
                            .span_stack
                            .iter()
                            .any(|s| s.role == SpanRole::Background);

                        let is_background_syllable = ended_span_ctx.role == SpanRole::Background
                            || is_within_background_container;

                        let target_syllables = if is_background_syllable {
                            &mut meta_state.current_bg_syllables
                        } else {
                            &mut meta_state.current_main_syllables
                        };

                        process_syllable(
                            start_ms,
                            end_ms,
                            &raw_text,
                            is_background_syllable,
                            &mut state.text_processing_buffer,
                            target_syllables,
                        );
                    } else if !raw_text.trim().is_empty() {
                        let is_within_background_container = meta_state
                            .span_stack
                            .iter()
                            .any(|s| s.role == SpanRole::Background);

                        let is_background_span = ended_span_ctx.role == SpanRole::Background
                            || is_within_background_container;

                        if is_background_span {
                            meta_state.current_bg_plain_text.push_str(&raw_text);
                        } else {
                            meta_state.current_main_plain_text.push_str(&raw_text);
                        }
                    }
                }
            }
            TAG_TEXT => {
                if let MetadataContext::InAuxiliaryText {
                    aux_type,
                    lang,
                    key: Some(text_key),
                } = &meta_state.context
                {
                    let main_plain_text = meta_state.current_main_plain_text.trim();
                    let bg_plain_text = meta_state.current_bg_plain_text.trim();
                    let has_plain_text = !main_plain_text.is_empty() || !bg_plain_text.is_empty();

                    let has_main_syllables = !meta_state.current_main_syllables.is_empty();
                    let has_bg_syllables = !meta_state.current_bg_syllables.is_empty();

                    // 判断是逐行翻译，还是带时间的辅助轨道
                    if !has_main_syllables
                        && !has_bg_syllables
                        && has_plain_text
                        && matches!(aux_type, AuxTrackType::Translation)
                    {
                        // 是逐行翻译，存入 line_translation_map
                        let line_translation = LineTranslation {
                            main: if main_plain_text.is_empty() {
                                None
                            } else {
                                Some(main_plain_text.to_string())
                            },
                            background: if bg_plain_text.is_empty() {
                                None
                            } else {
                                Some(bg_plain_text.to_string())
                            },
                        };

                        meta_state
                            .line_translation_map
                            .insert(text_key.clone(), (line_translation, lang.clone()));
                    } else if has_main_syllables || has_bg_syllables {
                        // 是带时间戳的辅助轨道，存入 timed_track_map。
                        let entry = meta_state
                            .timed_track_map
                            .entry(text_key.clone())
                            .or_default();
                        let mut metadata = HashMap::new();
                        if let Some(language) = lang {
                            metadata.insert(TrackMetadataKey::Language, language.clone());
                        }

                        if has_main_syllables {
                            let track = LyricTrack {
                                words: vec![Word {
                                    syllables: std::mem::take(
                                        &mut meta_state.current_main_syllables,
                                    ),
                                    ..Default::default()
                                }],
                                metadata: metadata.clone(),
                            };
                            let target_set = &mut entry.main_tracks;
                            match aux_type {
                                AuxTrackType::Translation => target_set.translations.push(track),
                                AuxTrackType::Romanization => target_set.romanizations.push(track),
                            }
                        }
                        if has_bg_syllables {
                            let track = LyricTrack {
                                words: vec![Word {
                                    syllables: std::mem::take(&mut meta_state.current_bg_syllables),
                                    ..Default::default()
                                }],
                                metadata: metadata.clone(),
                            };
                            let target_set = &mut entry.background_tracks;
                            match aux_type {
                                AuxTrackType::Translation => target_set.translations.push(track),
                                AuxTrackType::Romanization => target_set.romanizations.push(track),
                            }
                        }
                    }

                    meta_state.current_main_plain_text.clear();
                    meta_state.current_bg_plain_text.clear();
                }
                // 回到上一级上下文
                if let MetadataContext::InAuxiliaryText { aux_type, lang, .. } = &meta_state.context
                {
                    meta_state.context = MetadataContext::InAuxiliaryEntry {
                        aux_type: *aux_type,
                        lang: lang.clone(),
                    };
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

/// 处理全局事件（在 `<p>` 或 `<metadata>` 之外的事件）。
/// 主要负责识别文档的根元素、body、div 和 p 的开始，并相应地更新状态。
fn handle_global_event(
    event: &Event<'_>,
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
            TAG_METADATA => state.in_metadata = true,

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

                let start_ms = get_time_attribute(e, reader, &[ATTR_BEGIN], warnings)?.unwrap_or(0);
                let end_ms = get_time_attribute(e, reader, &[ATTR_END], warnings)?.unwrap_or(0);

                let agent_attr_val =
                    get_string_attribute(e, reader, &[ATTR_AGENT, ATTR_AGENT_ALIAS])?;
                let mut final_agent_id = None;

                if let Some(val) = agent_attr_val {
                    // 检查这个值是否是 <head> 中已定义的 ID
                    if state.agent_store.agents_by_id.contains_key(&val) {
                        final_agent_id = Some(val);
                    } else {
                        // 值被视为一个直接的名称，需要为其创建/查找ID
                        if let Some(existing_id) = state.agent_name_to_id_map.get(&val) {
                            final_agent_id = Some(existing_id.clone());
                        } else {
                            state.agent_counter += 1;
                            let new_id = format!("v{}", state.agent_counter);

                            state
                                .agent_name_to_id_map
                                .insert(val.clone(), new_id.clone());

                            // 同时在 AgentStore 中创建一个新的 Agent 记录
                            let new_agent = Agent {
                                id: new_id.clone(),
                                name: Some(val),
                                agent_type: AgentType::Person, // 默认为 Person
                            };
                            state
                                .agent_store
                                .agents_by_id
                                .insert(new_id.clone(), new_agent);

                            final_agent_id = Some(new_id);
                        }
                    }
                }

                let song_part = get_string_attribute(e, reader, &[ATTR_ITUNES_SONG_PART])?
                    .or(state.body_state.current_div_song_part.clone());
                let itunes_key = get_string_attribute(e, reader, &[ATTR_ITUNES_KEY])?;

                state.body_state.current_p_element_data = Some(CurrentPElementData {
                    start_ms,
                    end_ms,
                    agent: final_agent_id,
                    song_part,
                    itunes_key,
                    ..Default::default()
                });

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

/// 处理 `</p>` 结束事件。
/// 在此事件中，会回填来自 <iTunesMetadata> 的逐行翻译
fn handle_p_end(
    state: &mut TtmlParserState,
    lines: &mut Vec<LyricLine>,
    warnings: &mut Vec<String>,
) {
    if let Some(mut p_data) = state.body_state.current_p_element_data.take() {
        if let Some(key) = &p_data.itunes_key {
            // 回填逐行翻译
            if let Some((line_translation, lang)) =
                state.metadata_state.line_translation_map.get(key)
            {
                // 处理主音轨翻译
                if let Some(main_text) = &line_translation.main
                    && let Some(main_annotated_track) = p_data
                        .tracks_accumulator
                        .iter_mut()
                        .find(|at| at.content_type == ContentType::Main)
                {
                    // 检查是否已存在具有相同文本的翻译轨道
                    let translation_exists =
                        main_annotated_track.translations.iter().any(|track| {
                            track
                                .words
                                .iter()
                                .flat_map(|w| &w.syllables)
                                .any(|s| s.text == *main_text)
                        });

                    if !translation_exists {
                        let translation_track =
                            create_simple_translation_track(main_text, lang.as_ref());
                        main_annotated_track.translations.push(translation_track);
                    }
                }

                // 处理背景人声音轨的翻译
                if let Some(bg_text) = &line_translation.background {
                    let bg_annotated_track =
                        get_or_create_target_annotated_track(&mut p_data, ContentType::Background);

                    let translation_exists = bg_annotated_track.translations.iter().any(|track| {
                        track
                            .words
                            .iter()
                            .flat_map(|w| &w.syllables)
                            .any(|s| s.text == *bg_text)
                    });

                    if !translation_exists {
                        let translation_track =
                            create_simple_translation_track(bg_text, lang.as_ref());
                        bg_annotated_track.translations.push(translation_track);
                    }
                }
            }
        }
        finalize_p_element(p_data, lines, state, warnings);
    }
    // 重置 p 内部的状态
    state.body_state.in_p = false;
    state.body_state.span_stack.clear();
    state.body_state.last_syllable_info = LastSyllableInfo::None;
}

/// 处理在 `<p>` 标签内部的事件。
fn handle_p_event(
    event: &Event<'_>,
    state: &mut TtmlParserState,
    reader: &Reader<&[u8]>,
    lines: &mut Vec<LyricLine>,
    warnings: &mut Vec<String>,
) -> Result<(), ConvertError> {
    match event {
        Event::Start(e) if e.local_name().as_ref() == TAG_SPAN => {
            process_span_start(e, state, reader, warnings)?;
        }
        Event::Text(e) => process_text_event(e, state)?,
        Event::GeneralRef(e) => {
            let entity_name = str::from_utf8(e.as_ref())
                .map_err(|err| ConvertError::Internal(format!("无法将实体名解码为UTF-8: {err}")))?;

            let decoded_char = if let Some(num_str) = entity_name.strip_prefix('#') {
                let (radix, code_point_str) = if let Some(stripped) = num_str.strip_prefix('x') {
                    (16, stripped)
                } else {
                    (10, num_str)
                };

                if let Ok(code_point) = u32::from_str_radix(code_point_str, radix) {
                    char::from_u32(code_point).unwrap_or('\0')
                } else {
                    warnings.push(format!("无法解析无效的XML数字实体 '&{entity_name};'"));
                    '\0'
                }
            } else {
                match entity_name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => {
                        warnings.push(format!("忽略了未知的XML实体 '&{entity_name};'"));
                        '\0'
                    }
                }
            };

            if decoded_char != '\0'
                && let Some(p_data) = state.body_state.current_p_element_data.as_mut()
            {
                if state.body_state.span_stack.is_empty() {
                    p_data.line_text_accumulator.push(decoded_char);
                } else {
                    state.text_buffer.push(decoded_char);
                }
            }
        }
        Event::End(e) => match e.local_name().as_ref() {
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
                handle_p_end(state, lines, warnings);
            }
            TAG_SPAN => {
                process_span_end(state, warnings)?;
            }
            _ => {}
        },
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
    warnings: &mut Vec<String>,
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
    let start_ms = get_time_attribute(e, reader, &[ATTR_BEGIN], warnings)?;
    let end_ms = get_time_attribute(e, reader, &[ATTR_END], warnings)?;

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
        && !p_data
            .tracks_accumulator
            .iter()
            .any(|t| t.content_type == ContentType::Background)
    {
        p_data.tracks_accumulator.push(AnnotatedTrack {
            content_type: ContentType::Background,
            content: LyricTrack::default(),
            translations: vec![],
            romanizations: vec![],
        });
    }

    Ok(())
}

/// 处理文本事件。
/// 这个函数的核心逻辑是区分 "音节间的空格" 和 "音节内的文本"。
fn process_text_event(e_text: &BytesText, state: &mut TtmlParserState) -> Result<(), ConvertError> {
    let text_slice = e_text.xml_content()?;

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
            let target_content_type = if was_background {
                ContentType::Background
            } else {
                ContentType::Main
            };

            if let Some(last_syl) = p_data
                .tracks_accumulator
                .iter_mut()
                .find(|t| t.content_type == target_content_type)
                .and_then(|at| at.content.words.last_mut())
                .and_then(|w| w.syllables.last_mut())
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
    // 从堆栈中弹出刚刚结束的 span 的上下文
    if let Some(ended_span_ctx) = state.body_state.span_stack.pop() {
        // 获取并清空缓冲区中的文本
        let raw_text_from_buffer = std::mem::take(&mut state.text_buffer);

        // 根据 span 的角色分发给不同的处理器
        match ended_span_ctx.role {
            SpanRole::Generic => {
                handle_generic_span_end(state, &ended_span_ctx, &raw_text_from_buffer, warnings)?;
            }
            SpanRole::Translation | SpanRole::Romanization => {
                handle_auxiliary_span_end(state, &ended_span_ctx, &raw_text_from_buffer)?;
            }
            SpanRole::Background => {
                handle_background_span_end(
                    state,
                    &ended_span_ctx,
                    &raw_text_from_buffer,
                    warnings,
                )?;
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
    if let (Some(start_ms), Some(end_ms)) = (ctx.start_ms, ctx.end_ms) {
        if text.is_empty() {
            return Ok(());
        }

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

        let target_content_type = if was_within_bg {
            ContentType::Background
        } else {
            ContentType::Main
        };

        let target_annotated_track =
            get_or_create_target_annotated_track(p_data, target_content_type);
        let target_content_track = &mut target_annotated_track.content;

        if target_content_track.words.is_empty() {
            target_content_track.words.push(Word::default());
        }
        let target_word = target_content_track.words.first_mut().unwrap();

        process_syllable(
            start_ms,
            end_ms.max(start_ms),
            text,
            was_within_bg,
            &mut state.text_processing_buffer,
            &mut target_word.syllables,
        );

        if !target_word.syllables.is_empty() {
            state.body_state.last_syllable_info = LastSyllableInfo::EndedSyllable {
                was_background: was_within_bg,
            };
        }
    } else if !text.trim().is_empty() {
        if state.is_line_timing_mode {
            if let Some(p_data) = state.body_state.current_p_element_data.as_mut() {
                if !p_data.line_text_accumulator.is_empty()
                    && !p_data.line_text_accumulator.ends_with(char::is_whitespace)
                {
                    p_data.line_text_accumulator.push(' ');
                }
                p_data.line_text_accumulator.push_str(text.trim());
            }
        } else {
            warnings.push(format!(
                "逐字模式下，span缺少时间信息，文本 '{}' 被忽略。",
                text.trim().escape_debug()
            ));
        }
    }
    Ok(())
}

fn process_syllable(
    start_ms: u64,
    end_ms: u64,
    raw_text: &str,
    is_background: bool,
    text_processing_buffer: &mut String,
    syllable_accumulator: &mut Vec<LyricSyllable>,
) {
    // 处理前导空格
    if raw_text.starts_with(char::is_whitespace)
        && let Some(prev_syllable) = syllable_accumulator.last_mut()
        && !prev_syllable.ends_with_space
    {
        prev_syllable.ends_with_space = true;
    }

    let trimmed_text = raw_text.trim();
    if trimmed_text.is_empty() {
        return;
    }

    // 根据是否为背景人声，对文本进行清理
    text_processing_buffer.clear();
    if is_background {
        clean_parentheses_from_bg_text_into(trimmed_text, text_processing_buffer);
    } else {
        normalize_text_whitespace_into(trimmed_text, text_processing_buffer);
    }

    if text_processing_buffer.is_empty() {
        return;
    }

    // 创建新的音节
    let new_syllable = LyricSyllable {
        text: text_processing_buffer.clone(),
        start_ms,
        end_ms,
        duration_ms: Some(end_ms.saturating_sub(start_ms)),
        ends_with_space: raw_text.ends_with(char::is_whitespace),
    };

    syllable_accumulator.push(new_syllable);
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

    let was_within_bg = state
        .body_state
        .span_stack
        .iter()
        .any(|s| s.role == SpanRole::Background);

    let target_content_type = if was_within_bg {
        ContentType::Background
    } else {
        ContentType::Main
    };

    let target_annotated_track = get_or_create_target_annotated_track(p_data, target_content_type);

    let syllable = LyricSyllable {
        text: state.text_processing_buffer.clone(),
        ..Default::default()
    };
    let word = Word {
        syllables: vec![syllable],
        ..Default::default()
    };
    let mut metadata = HashMap::new();

    let mut aux_track = LyricTrack {
        words: vec![word],
        metadata: HashMap::default(),
    };

    match ctx.role {
        SpanRole::Translation => {
            if let Some(lang) = ctx.lang.clone().or(state.default_translation_lang.clone()) {
                metadata.insert(TrackMetadataKey::Language, lang);
            }
            aux_track.metadata = metadata;
            target_annotated_track.translations.push(aux_track);
        }
        SpanRole::Romanization => {
            if let Some(lang) = ctx.lang.clone().or(state.default_romanization_lang.clone()) {
                metadata.insert(TrackMetadataKey::Language, lang);
            }
            if let Some(scheme) = ctx.scheme.clone() {
                metadata.insert(TrackMetadataKey::Scheme, scheme);
            }
            aux_track.metadata = metadata;
            target_annotated_track.romanizations.push(aux_track);
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

    // 处理不规范的情况：背景容器直接包含文本，而不是通过嵌套的 span。
    let trimmed_text = text.trim();
    if !trimmed_text.is_empty() {
        if let (Some(start_ms), Some(end_ms)) = (ctx.start_ms, ctx.end_ms) {
            if let Some(bg_annotated_track) = p_data
                .tracks_accumulator
                .iter_mut()
                .find(|t| t.content_type == ContentType::Background)
            {
                let bg_content_track = &mut bg_annotated_track.content;
                // 只有在背景容器内部没有其他音节时，才将此直接文本视为一个音节
                if bg_content_track.words.is_empty()
                    || bg_content_track
                        .words
                        .iter()
                        .all(|w| w.syllables.is_empty())
                {
                    clean_parentheses_from_bg_text_into(
                        trimmed_text,
                        &mut state.text_processing_buffer,
                    );

                    let syllable = LyricSyllable {
                        text: state.text_processing_buffer.clone(),
                        start_ms,
                        end_ms: end_ms.max(start_ms),
                        duration_ms: Some(end_ms.saturating_sub(start_ms)),
                        ends_with_space: !text.is_empty() && text.ends_with(char::is_whitespace),
                    };

                    if bg_content_track.words.is_empty() {
                        bg_content_track.words.push(Word::default());
                    }
                    bg_content_track
                        .words
                        .first_mut()
                        .unwrap()
                        .syllables
                        .push(syllable);

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
    mut p_data: CurrentPElementData,
    lines: &mut Vec<LyricLine>,
    state: &mut TtmlParserState,
    _warnings: &mut Vec<String>,
) {
    let main_track_has_syllables = p_data
        .tracks_accumulator
        .iter()
        .find(|at| at.content_type == ContentType::Main)
        .is_some_and(|at| !at.content.words.iter().all(|w| w.syllables.is_empty()));

    if !main_track_has_syllables && !p_data.line_text_accumulator.trim().is_empty() {
        if !p_data
            .tracks_accumulator
            .iter()
            .any(|at| at.content_type == ContentType::Main)
        {
            p_data.tracks_accumulator.insert(
                0,
                AnnotatedTrack {
                    content_type: ContentType::Main,
                    ..Default::default()
                },
            );
        }

        let main_annotated_track = p_data
            .tracks_accumulator
            .iter_mut()
            .find(|at| at.content_type == ContentType::Main)
            .unwrap(); // 已经在上面保证了它一定存在

        normalize_text_whitespace_into(
            &p_data.line_text_accumulator,
            &mut state.text_processing_buffer,
        );
        if !state.text_processing_buffer.is_empty() {
            let syllable = LyricSyllable {
                text: state.text_processing_buffer.clone(),
                start_ms: p_data.start_ms,
                end_ms: p_data.end_ms,
                ..Default::default()
            };
            main_annotated_track.content.words = vec![Word {
                syllables: vec![syllable],
                ..Default::default()
            }];
        }
    }

    if let Some(key) = &p_data.itunes_key
        && let Some(detailed_tracks) = state.metadata_state.timed_track_map.get(key)
    {
        if let Some(main_annotated_track) = p_data
            .tracks_accumulator
            .iter_mut()
            .find(|at| at.content_type == ContentType::Main)
        {
            main_annotated_track
                .translations
                .extend(detailed_tracks.main_tracks.translations.clone());
            main_annotated_track
                .romanizations
                .extend(detailed_tracks.main_tracks.romanizations.clone());
        }
        if let Some(bg_annotated_track) = p_data
            .tracks_accumulator
            .iter_mut()
            .find(|at| at.content_type == ContentType::Background)
        {
            bg_annotated_track
                .translations
                .extend(detailed_tracks.background_tracks.translations.clone());
            bg_annotated_track
                .romanizations
                .extend(detailed_tracks.background_tracks.romanizations.clone());
        }
    }

    let mut new_line = LyricLine {
        start_ms: p_data.start_ms,
        end_ms: p_data.end_ms,
        agent: p_data.agent,
        song_part: p_data.song_part,
        tracks: p_data.tracks_accumulator,
        itunes_key: p_data.itunes_key.clone(),
    };

    // 重新计算行的结束时间，应为所有轨道中所有音节的最大结束时间
    let max_track_end_ms = new_line
        .tracks
        .iter()
        .flat_map(|at| {
            let content_syllables = at.content.words.iter().flat_map(|w| &w.syllables);
            let translation_syllables = at
                .translations
                .iter()
                .flat_map(|t| t.words.iter().flat_map(|w| &w.syllables));
            let romanization_syllables = at
                .romanizations
                .iter()
                .flat_map(|t| t.words.iter().flat_map(|w| &w.syllables));
            content_syllables
                .chain(translation_syllables)
                .chain(romanization_syllables)
        })
        .map(|syllable| syllable.end_ms)
        .max()
        .unwrap_or(0);

    new_line.end_ms = new_line.end_ms.max(max_track_end_ms);

    let is_empty = new_line.tracks.iter().all(|at| {
        at.content.words.iter().all(|w| w.syllables.is_empty())
            && at.translations.is_empty()
            && at.romanizations.is_empty()
    });

    if !is_empty {
        lines.push(new_line);
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
        Ok(val * 10u64.pow(3 - u32::try_from(ms_str.len()).unwrap_or(3)))
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
    warnings: &mut Vec<String>,
) -> Result<Option<u64>, ConvertError> {
    if let Some(value_str) = get_string_attribute(e, reader, attr_names)? {
        match parse_ttml_time_to_ms(&value_str) {
            Ok(ms) => Ok(Some(ms)),
            Err(err) => {
                warnings.push(format!(
                    "时间戳 '{value_str}' 解析失败 ({err}). 该时间戳将被忽略."
                ));
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// 尝试从一个XML格式错误中恢复。
fn attempt_recovery_from_error(
    state: &mut TtmlParserState,
    reader: &Reader<&[u8]>,
    lines: &mut Vec<LyricLine>,
    warnings: &mut Vec<String>,
    error: &quick_xml::errors::Error,
) {
    let position = reader.error_position();
    warnings.push(format!("TTML 格式错误，位置 {position}: {error}。"));

    if state.body_state.in_p {
        // 错误发生在 <p> 标签内部
        // 尝试抢救当前行的数据，然后跳出这个<p>
        warnings.push(format!(
            "错误发生在 <p> 元素内部 (开始于 {}ms)。尝试恢复已经解析的数据。",
            state
                .body_state
                .current_p_element_data
                .as_ref()
                .map_or(0, |d| d.start_ms)
        ));

        // 处理和保存当前 <p> 中已经累积的数据
        // 把current_p_element_data中的内容（即使不完整）转换成一个 LyricLine
        handle_p_end(state, lines, warnings);

        // handle_p_end 已经将 in_p 设为 false，并清理了 span 栈，
        // 我们现在回到了“p之外，body之内”的安全状态
    } else if state.in_metadata {
        // 错误发生在 <metadata> 内部
        // 元数据太复杂了，简单地放弃所有数据好了
        warnings.push("错误发生在 <metadata> 块内部。放弃所有元数据。".to_string());
        state.in_metadata = false;
        state.metadata_state = MetadataParseState::default();
    } else {
        // 错误发生在全局作用域
        // 可能是 <body> 或 <div> 标签损坏。恢复的把握较小。
        // 我们重置所有 body 相关的状态，期望能找到下一个有效的 <p>。
        warnings
            .push("错误发生在全局作用域。将重置解析器状态，尝试寻找下一个有效元素。".to_string());
        state.body_state = BodyParseState::default();
    }
}

fn get_or_create_target_annotated_track(
    p_data: &mut CurrentPElementData,
    content_type: ContentType,
) -> &mut AnnotatedTrack {
    if let Some(index) = p_data
        .tracks_accumulator
        .iter()
        .position(|t| t.content_type == content_type)
    {
        &mut p_data.tracks_accumulator[index]
    } else {
        p_data.tracks_accumulator.push(AnnotatedTrack {
            content_type,
            ..Default::default()
        });
        p_data.tracks_accumulator.last_mut().unwrap() // 刚插入，所以 unwrap 是安全的
    }
}

fn create_simple_translation_track(text: &str, lang: Option<&String>) -> LyricTrack {
    let syllable = LyricSyllable {
        text: text.to_string(),
        ..Default::default()
    };
    let word = Word {
        syllables: vec![syllable],
        ..Default::default()
    };
    let mut metadata = HashMap::new();
    if let Some(lang_code) = lang {
        metadata.insert(TrackMetadataKey::Language, lang_code.clone());
    }
    LyricTrack {
        words: vec![word],
        metadata,
    }
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
        assert_eq!(parse_ttml_time_to_ms("99999.123s").unwrap(), 99_999_123);
        assert_eq!(parse_ttml_time_to_ms("01:02:03.456").unwrap(), 3_723_456);
        assert_eq!(parse_ttml_time_to_ms("05:10.1").unwrap(), 310_100);
        assert_eq!(parse_ttml_time_to_ms("05:10.12").unwrap(), 310_120);
        assert_eq!(parse_ttml_time_to_ms("7.123").unwrap(), 7123);
        assert_eq!(parse_ttml_time_to_ms("7").unwrap(), 7000);
        assert_eq!(parse_ttml_time_to_ms("15.5s").unwrap(), 15500);
        assert_eq!(parse_ttml_time_to_ms("15s").unwrap(), 15000);

        assert_eq!(parse_ttml_time_to_ms("0").unwrap(), 0);
        assert_eq!(parse_ttml_time_to_ms("0.0s").unwrap(), 0);
        assert_eq!(parse_ttml_time_to_ms("00:00:00.000").unwrap(), 0);
        assert_eq!(parse_ttml_time_to_ms("99:59:59.999").unwrap(), 359_999_999);
        assert_eq!(parse_ttml_time_to_ms("60").unwrap(), 60000);
        assert_eq!(parse_ttml_time_to_ms("123.456").unwrap(), 123_456);

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

    // 如果你期望看到集成测试，请前往 tests\ttml_parser_integration_tests.rs
}
