//! 定义了歌词转换中使用的核心数据类型。

use std::{collections::HashMap, fmt, io, str::FromStr};

use quick_xml::{
    Error as QuickXmlErrorMain, encoding::EncodingError,
    events::attributes::AttrError as QuickXmlAttrError,
};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use thiserror::Error;

use crate::metadata_processor::MetadataStore;

//=============================================================================
// 1. 错误枚举
//=============================================================================

/// 定义歌词转换和处理过程中可能发生的各种错误。
#[derive(Error, Debug)]
pub enum ConvertError {
    /// XML 生成错误，通常来自 `quick-xml` 库。
    #[error("生成 XML 错误: {0}")]
    Xml(#[from] QuickXmlErrorMain),
    /// XML 属性解析错误，通常来自 `quick-xml` 库。
    #[error("XML 属性错误: {0}")]
    Attribute(#[from] QuickXmlAttrError),
    /// 整数解析错误。
    #[error("解析错误: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    /// 无效的时间格式字符串。
    #[error("无效的时间格式: {0}")]
    InvalidTime(String),
    /// 字符串格式化错误。
    #[error("格式错误: {0}")]
    Format(#[from] fmt::Error),
    /// 内部逻辑错误或未明确分类的错误。
    #[error("错误: {0}")]
    Internal(String),
    /// 文件读写等IO错误。
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),
    /// 从字节序列转换为 UTF-8 字符串失败。
    #[error("UTF-8 转换错误: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// 无效的歌词格式。
    #[error("无效的歌词格式: {0}")]
    InvalidLyricFormat(String),
    /// XML 文本编码或解码错误。
    #[error("文本编码或解码错误: {0}")]
    Encoding(#[from] EncodingError),
    /// 词组边界检测错误
    #[error("词组边界检测失败: {0}")]
    WordBoundaryDetection(String),
    /// 振假名解析错误
    #[error("振假名解析失败: {0}")]
    FuriganaParsingError(String),
    /// 轨道合并错误
    #[error("轨道合并失败: {0}")]
    TrackMergeError(String),
}

impl From<ConvertError> for std::io::Error {
    fn from(err: ConvertError) -> Self {
        std::io::Error::other(err)
    }
}

/// 定义从字符串解析 `CanonicalMetadataKey` 时可能发生的错误。
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParseCanonicalMetadataKeyError(String); // 存储无法解析的原始键字符串

impl fmt::Display for ParseCanonicalMetadataKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "未知或无效的元数据键: {}", self.0)
    }
}
impl std::error::Error for ParseCanonicalMetadataKeyError {}

//=============================================================================
// 2. 核心歌词格式枚举及相关
//=============================================================================

/// 枚举：表示支持的歌词格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, Serialize, Deserialize, EnumIter)]
#[strum(ascii_case_insensitive)]
#[derive(Default)]
pub enum LyricFormat {
    /// `Timed Text Markup Language` 格式。
    #[default]
    Ttml,
}

//=============================================================================
// 3. 歌词内部表示结构
//=============================================================================

/// 定义可以被注解的内容轨道类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ContentType {
    #[default]
    /// 主歌词
    Main,
    /// 背景人声
    Background,
}

/// 定义轨道元数据的规范化键。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrackMetadataKey {
    /// BCP 47 语言代码
    Language,
    /// 罗马音方案名
    Scheme,
    /// 自定义元数据键
    Custom(String),
}

/// 表示振假名中的一个音节。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuriganaSyllable {
    /// 振假名文本内容
    pub text: String,
    /// 可选的时间戳 (`start_ms`, `end_ms`)
    pub timing: Option<(u64, u64)>,
}

/// 表示一个语义上的"单词"或"词组"，主要为振假名服务。
///
/// 目前还没有歌词格式提供词组信息，应将整行直接作为一个词组。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    /// 组成该词的音节列表
    pub syllables: Vec<LyricSyllable>,
    /// 可选的振假名信息
    pub furigana: Option<Vec<FuriganaSyllable>>,
}

/// 一个通用的歌词轨道。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricTrack {
    /// 组成该轨道的音节列表。
    pub words: Vec<Word>,
    /// 轨道元数据。
    #[serde(default)]
    pub metadata: HashMap<TrackMetadataKey, String>,
}

/// 将一个内容轨道（如主歌词）及其所有注解轨道（如翻译、罗马音）绑定在一起的结构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AnnotatedTrack {
    /// 该内容轨道的类型。
    pub content_type: ContentType,

    /// 内容轨道本身。
    pub content: LyricTrack,

    /// 依附于该内容轨道的翻译轨道列表。
    #[serde(default)]
    pub translations: Vec<LyricTrack>,

    /// 依附于该内容轨道的罗马音轨道列表。
    #[serde(default)]
    pub romanizations: Vec<LyricTrack>,
}

/// 表示一位演唱者的类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentType {
    #[default]
    /// 单人演唱。
    Person,
    /// 合唱。
    Group,
    /// 未指定或其它类型。
    Other,
}

/// 表示歌词中的演唱者。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// 内部ID, 例如 "v1"
    pub id: String,
    /// 可选的完整名称，例如 "演唱者1号"
    pub name: Option<String>,
    /// Agent 的类型
    pub agent_type: AgentType,
}

/// 用于存储歌词轨道中识别到的所有演唱者。
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStore {
    /// 从 ID 到演唱者结构体的映射。
    pub agents_by_id: HashMap<String, Agent>,
}

impl AgentStore {
    /// 从歌词行列表中构建 `AgentStore`
    #[must_use]
    pub fn from_metadata_store(metadata_store: &MetadataStore) -> Self {
        let mut store = AgentStore::default();

        if let Some(agent_definitions) = metadata_store.get_multiple_values_by_key("agent") {
            for def_string in agent_definitions {
                let (id, parsed_name) = match def_string.split_once('=') {
                    Some((id, name)) => (id.to_string(), Some(name.to_string())),
                    None => (def_string.clone(), None),
                };

                let is_chorus = id == "v1000"
                    || parsed_name.as_deref() == Some("合")
                    || parsed_name.as_deref() == Some("合唱");

                let final_name = if is_chorus { None } else { parsed_name };
                let agent_type = if is_chorus {
                    AgentType::Group
                } else {
                    AgentType::Person
                };

                let agent = Agent {
                    id: id.clone(),
                    name: final_name,
                    agent_type,
                };
                store.agents_by_id.insert(id, agent);
            }
        }
        store
    }

    /// 获取所有 Agent 的迭代器
    pub fn all_agents(&self) -> impl Iterator<Item = &Agent> {
        self.agents_by_id.values()
    }
}

/// 歌词行结构，作为多个并行带注解轨道的容器。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricLine {
    /// 该行包含的所有带注解的轨道。
    pub tracks: Vec<AnnotatedTrack>,
    /// 行的开始时间，相对于歌曲开始的绝对时间（毫秒）。
    pub start_ms: u64,
    /// 行的结束时间，相对于歌曲开始的绝对时间（毫秒）。
    pub end_ms: u64,
    /// 可选的演唱者标识。
    ///
    /// 应该为数字 ID，例如 "v1"，"v1000"。
    pub agent: Option<String>,
    /// 可选的歌曲组成部分标记。
    pub song_part: Option<String>,
    /// 可选的 iTunes Key (如 "L1", "L2")。
    pub itunes_key: Option<String>,
}

impl LyricTrack {
    /// 将轨道内所有音节的文本拼接成一个完整的字符串。
    #[must_use]
    pub fn text(&self) -> String {
        self.words
            .iter()
            .flat_map(|word| &word.syllables)
            .map(|syl| {
                if syl.ends_with_space {
                    format!("{} ", syl.text)
                } else {
                    syl.text.clone()
                }
            })
            .collect::<String>()
            .trim_end()
            .to_string()
    }
}

impl LyricLine {
    /// 创建一个带有指定时间戳的空 `LyricLine`。
    #[must_use]
    pub fn new(start_ms: u64, end_ms: u64) -> Self {
        Self {
            start_ms,
            end_ms,
            ..Default::default()
        }
    }

    /// 返回一个迭代器，用于遍历所有指定内容类型的带注解轨道。
    pub fn tracks_by_type(
        &self,
        content_type: ContentType,
    ) -> impl Iterator<Item = &AnnotatedTrack> {
        self.tracks
            .iter()
            .filter(move |t| t.content_type == content_type)
    }

    /// 返回一个迭代器，用于遍历所有主歌词轨道 (`ContentType::Main`)。
    pub fn main_tracks(&self) -> impl Iterator<Item = &AnnotatedTrack> {
        self.tracks_by_type(ContentType::Main)
    }

    /// 返回一个迭代器，用于遍历所有背景人声音轨 (`ContentType::Background`)。
    pub fn background_tracks(&self) -> impl Iterator<Item = &AnnotatedTrack> {
        self.tracks_by_type(ContentType::Background)
    }

    /// 获取第一个主歌词轨道（如果存在）。
    #[must_use]
    pub fn main_track(&self) -> Option<&AnnotatedTrack> {
        self.main_tracks().next()
    }

    /// 获取第一个背景人声音轨（如果存在）。
    #[must_use]
    pub fn background_track(&self) -> Option<&AnnotatedTrack> {
        self.background_tracks().next()
    }

    /// 获取第一个主歌词轨道的完整文本（如果存在）。
    #[must_use]
    pub fn main_text(&self) -> Option<String> {
        self.main_track().map(|t| t.content.text())
    }

    /// 获取第一个背景人声轨道的完整文本（如果存在）。
    #[must_use]
    pub fn background_text(&self) -> Option<String> {
        self.background_track().map(|t| t.content.text())
    }

    /// 向该行添加一个预先构建好的带注解轨道。
    pub fn add_track(&mut self, track: AnnotatedTrack) {
        self.tracks.push(track);
    }

    /// 向该行添加一个新的、简单的内容轨道（主歌词或背景）。
    ///
    /// # 参数
    /// * `content_type` - 轨道的类型 (`Main` 或 `Background`)。
    /// * `text` - 该轨道的完整文本。
    pub fn add_content_track(&mut self, content_type: ContentType, text: impl Into<String>) {
        let syllable = LyricSyllable {
            text: text.into(),
            start_ms: self.start_ms,
            end_ms: self.end_ms,
            ..Default::default()
        };
        let track = AnnotatedTrack {
            content_type,
            content: LyricTrack {
                words: vec![Word {
                    syllables: vec![syllable],
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        self.add_track(track);
    }

    /// 为该行中所有指定类型的内容轨道添加一个翻译。
    /// 例如，可用于为所有主歌词轨道添加一个统一的翻译。
    pub fn add_translation(
        &mut self,
        content_type: ContentType,
        text: impl Into<String>,
        language: Option<&str>,
    ) {
        let text = text.into();
        for track in self
            .tracks
            .iter_mut()
            .filter(|t| t.content_type == content_type)
        {
            let mut metadata = HashMap::new();
            if let Some(lang) = language {
                metadata.insert(TrackMetadataKey::Language, lang.to_string());
            }
            let translation_track = LyricTrack {
                words: vec![Word {
                    syllables: vec![LyricSyllable {
                        text: text.clone(),
                        start_ms: self.start_ms,
                        end_ms: self.end_ms,
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                metadata,
            };
            track.translations.push(translation_track);
        }
    }

    /// 为该行中所有指定类型的内容轨道添加一个罗马音。
    pub fn add_romanization(
        &mut self,
        content_type: ContentType,
        text: impl Into<String>,
        scheme: Option<&str>,
    ) {
        let text = text.into();
        for track in self
            .tracks
            .iter_mut()
            .filter(|t| t.content_type == content_type)
        {
            let mut metadata = HashMap::new();
            if let Some(s) = scheme {
                metadata.insert(TrackMetadataKey::Scheme, s.to_string());
            }
            let romanization_track = LyricTrack {
                words: vec![Word {
                    syllables: vec![LyricSyllable {
                        text: text.clone(),
                        start_ms: self.start_ms,
                        end_ms: self.end_ms,
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                metadata,
            };
            track.romanizations.push(romanization_track);
        }
    }

    /// 移除所有指定类型的内容轨道及其所有注解。
    pub fn clear_tracks(&mut self, content_type: ContentType) {
        self.tracks.retain(|t| t.content_type != content_type);
    }
}

/// 通用的歌词音节结构，用于表示逐字歌词中的一个音节。
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricSyllable {
    /// 音节的文本内容。
    pub text: String,
    /// 音节开始时间，相对于歌曲开始的绝对时间（毫秒）。
    pub start_ms: u64,
    /// 音节结束时间，相对于歌曲开始的绝对时间（毫秒）。
    pub end_ms: u64,
    /// 可选的音节持续时间（毫秒）。
    /// 如果提供，`end_ms` 可以由 `start_ms + duration_ms` 计算得出，反之亦然。
    /// 解析器应确保 `start_ms` 和 `end_ms` 最终被填充。
    pub duration_ms: Option<u64>,
    /// 指示该音节后是否应有空格。
    pub ends_with_space: bool,
}

//=============================================================================
// 4. 元数据结构体
//=============================================================================

/// 定义元数据的规范化键。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CanonicalMetadataKey {
    /// 歌曲标题。
    Title,
    /// 艺术家。
    Artist,
    /// 专辑名。
    Album,
    /// 主歌词的语言代码 (BCP 47)。
    Language,
    /// 全局时间偏移量（毫秒）。
    Offset,
    /// 词曲作者。
    Songwriter,
    /// 网易云音乐 ID。
    NcmMusicId,
    /// QQ音乐 ID。
    QqMusicId,
    /// Spotify ID。
    SpotifyId,
    /// Apple Music ID。
    AppleMusicId,
    /// 国际标准音像制品编码 (International Standard Recording Code)。
    Isrc,
    /// 逐词歌词作者 Github ID。
    TtmlAuthorGithub,
    /// 逐词歌词作者 GitHub 用户名。
    TtmlAuthorGithubLogin,

    /// 用于所有其他未明确定义的标准或非标准元数据键。
    Custom(String),
}

impl fmt::Display for CanonicalMetadataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key_name = match self {
            CanonicalMetadataKey::Title => "Title",
            CanonicalMetadataKey::Artist => "Artist",
            CanonicalMetadataKey::Album => "Album",
            CanonicalMetadataKey::Language => "Language",
            CanonicalMetadataKey::Offset => "Offset",
            CanonicalMetadataKey::Songwriter => "Songwriter",
            CanonicalMetadataKey::NcmMusicId => "NcmMusicId",
            CanonicalMetadataKey::QqMusicId => "QqMusicId",
            CanonicalMetadataKey::SpotifyId => "SpotifyId",
            CanonicalMetadataKey::AppleMusicId => "AppleMusicId",
            CanonicalMetadataKey::Isrc => "Isrc",
            CanonicalMetadataKey::TtmlAuthorGithub => "TtmlAuthorGithub",
            CanonicalMetadataKey::TtmlAuthorGithubLogin => "TtmlAuthorGithubLogin",
            CanonicalMetadataKey::Custom(s) => s.as_str(),
        };
        write!(f, "{key_name}")
    }
}

impl CanonicalMetadataKey {
    /// 定义哪些键应该被显示出来
    #[must_use]
    pub fn is_public(&self) -> bool {
        matches!(
            self,
            Self::Title
                | Self::Artist
                | Self::Album
                | Self::NcmMusicId
                | Self::QqMusicId
                | Self::SpotifyId
                | Self::AppleMusicId
                | Self::Isrc
                | Self::TtmlAuthorGithub
                | Self::TtmlAuthorGithubLogin
        )
    }
}

impl FromStr for CanonicalMetadataKey {
    type Err = ParseCanonicalMetadataKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ti" | "title" | "musicname" => Ok(Self::Title),
            "ar" | "artist" | "artists" => Ok(Self::Artist),
            "al" | "album" => Ok(Self::Album),
            "by" | "ttmlauthorgithublogin" => Ok(Self::TtmlAuthorGithubLogin),
            "language" | "lang" => Ok(Self::Language),
            "offset" => Ok(Self::Offset),
            "songwriter" | "songwriters" => Ok(Self::Songwriter),
            "ncmmusicid" => Ok(Self::NcmMusicId),
            "qqmusicid" => Ok(Self::QqMusicId),
            "spotifyid" => Ok(Self::SpotifyId),
            "applemusicid" => Ok(Self::AppleMusicId),
            "isrc" => Ok(Self::Isrc),
            "ttmlauthorgithub" => Ok(Self::TtmlAuthorGithub),
            custom_key if !custom_key.is_empty() => Ok(Self::Custom(custom_key.to_string())),
            _ => Err(ParseCanonicalMetadataKeyError(s.to_string())),
        }
    }
}

//=============================================================================
// 5. 处理与数据结构体
//=============================================================================

/// 存储从源文件解析出的、准备进行进一步处理或转换的歌词数据。
/// 这是解析阶段的主要输出，也是后续处理和生成阶段的主要输入。
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedSourceData {
    /// 解析后的歌词行列表。
    pub lines: Vec<LyricLine>,
    /// 从文件头或特定元数据标签中解析出的原始（未规范化）元数据。
    /// 键是原始元数据标签名，值是该标签对应的所有值（因为某些标签可能出现多次）。
    pub raw_metadata: HashMap<String, Vec<String>>,
    /// 解析的源文件格式。
    pub source_format: LyricFormat,
    /// 从文件中解析出的所有演唱者信息。
    #[serde(default)]
    pub agents: AgentStore,
    /// 可选的原始文件名，可用于日志记录或某些特定转换逻辑。
    pub source_filename: Option<String>,
    /// 指示源文件是否是逐行歌词（例如LRC）。
    pub is_line_timed_source: bool,
    /// 解析过程中产生的警告信息列表。
    pub warnings: Vec<String>,
    /// 如果源文件是内嵌TTML的JSON，此字段存储原始的TTML字符串内容。
    pub raw_ttml_from_input: Option<String>,
    /// 指示输入的TTML（来自`raw_ttml_from_input`）是否被格式化。
    /// 这影响空格和换行的处理。
    pub detected_formatted_ttml_input: Option<bool>,
    /// 提供商名称
    pub source_name: String,
}

//=============================================================================
// 6. 辅助类型与函数
//=============================================================================

/// 表示从ASS中提取的标记信息。
/// 元组的第一个元素是原始行号，第二个元素是标记文本。
pub type MarkerInfo = (usize, String);

/// TTML 生成时的计时模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TtmlTimingMode {
    #[default]
    /// 逐字计时
    Word,
    /// 逐行计时
    Line,
}

/// TTML 解析选项
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TtmlParsingOptions {
    /// 当TTML本身未指定语言时，解析器可以使用的默认语言。
    #[serde(default)]
    pub default_languages: DefaultLanguageOptions,

    /// 强制指定计时模式，忽略文件内的 `itunes:timing` 属性和自动检测逻辑。
    #[serde(default)]
    pub force_timing_mode: Option<TtmlTimingMode>,
}

/// TTML 生成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtmlGenerationOptions {
    /// 生成的计时模式（逐字或逐行）。
    pub timing_mode: TtmlTimingMode,
    /// 指定输出 TTML 的主语言 (xml:lang)。如果为 None，则尝试从元数据推断。
    pub main_language: Option<String>,
    /// 为内联的翻译 `<span>` 指定默认语言代码。
    pub translation_language: Option<String>,
    /// 为内联的罗马音 `<span>` 指定默认语言代码。
    pub romanization_language: Option<String>,
    /// 是否遵循 Apple Music 的特定格式规则（例如，将翻译写入`<head>`而不是内联）。
    pub use_apple_format_rules: bool,
    /// 是否输出格式化的 TTML 文件。
    pub format: bool,
    /// 是否启用自动分词功能。
    pub auto_word_splitting: bool,
    /// 自动分词时，一个标点符号所占的权重（一个字符的权重为1.0）。
    pub punctuation_weight: f64,
}

impl Default for TtmlGenerationOptions {
    fn default() -> Self {
        Self {
            timing_mode: TtmlTimingMode::Word,
            main_language: None,
            translation_language: None,
            romanization_language: None,
            use_apple_format_rules: false,
            format: false,
            auto_word_splitting: false,
            punctuation_weight: 0.3,
        }
    }
}

/// TTML 解析时使用的默认语言选项
/// 当TTML本身未指定语言时，解析器可以使用这些值。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefaultLanguageOptions {
    /// 默认主语言代码
    pub main: Option<String>,
    /// 默认翻译语言代码
    pub translation: Option<String>,
    /// 默认罗马音语言代码
    pub romanization: Option<String>,
}

// =============================================================================
// 9. 平滑优化选项
// =============================================================================

/// 控制平滑优化的选项。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SyllableSmoothingOptions {
    /// 用于平滑的因子 (0.0 ~ 0.5)。
    pub factor: f64,
    /// 用于分组的时长差异阈值（毫秒）。
    pub duration_threshold_ms: u64,
    /// 用于分组的间隔阈值（毫秒）。
    pub gap_threshold_ms: u64,
    /// 组内平滑的次数。
    pub smoothing_iterations: u32,
}

impl Default for SyllableSmoothingOptions {
    fn default() -> Self {
        Self {
            factor: 0.15,
            duration_threshold_ms: 50,
            gap_threshold_ms: 100,
            smoothing_iterations: 5,
        }
    }
}
