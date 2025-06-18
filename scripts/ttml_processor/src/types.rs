//! types.rs
//!
//! 该模块是整个应用程序的数据模型中心，定义了所有模块共享的核心数据结构、
//! 枚举和错误类型。这些类型构成了歌词数据在内存中处理、转换和存储的
//! 标准化内部表示（Intermediate Representation, IR）。
//!
//! 模块内容划分如下：
//! 1. **错误类型**: 定义了在解析、转换和IO操作中可能遇到的所有错误。
//! 2. **格式定义**: 枚举了支持的歌词格式。
//! 3. **内部歌词表示**: 定义了从音节到歌词行的各级数据结构。
//! 4. **元数据结构**: 定义了规范化的元数据键和相关类型。
//! 5. **处理流程结构**: 定义了用于在不同处理阶段传递数据的聚合结构。
//! 6. **配置选项**: 定义了用于控制行为（如TTML生成）的选项结构。

// 引入所需的外部库和标准库模块
use quick_xml::Error as QuickXmlErrorMain;
use quick_xml::events::attributes::AttrError as QuickXmlAttrError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::str::FromStr;
use strum_macros::EnumString;
use thiserror::Error;

//=============================================================================
// 1. 错误枚举 (Error Enums)
//=============================================================================

/// 定义歌词转换和处理过程中可能发生的各种错误。
///
/// 使用 `thiserror` 宏可以方便地从其他错误类型进行转换（`#[from]`）
/// 并提供清晰的错误描述（`#[error(...)]`）。
#[derive(Error, Debug)]
pub enum ConvertError {
    /// XML 解析或生成错误，通常来自 `quick-xml` 库。
    #[error("生成或解析 XML 时出错: {0}")]
    Xml(#[from] QuickXmlErrorMain),

    /// XML 属性解析错误，当读取或解析标签属性时发生。
    #[error("处理 XML 属性时出错: {0}")]
    Attribute(#[from] QuickXmlAttrError),

    /// 字符串到整数的解析失败。
    #[error("解析整数失败: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// 时间戳字符串格式不符合 TTML 或其他预期标准。
    #[error("无效的时间格式: {0}")]
    InvalidTime(String),

    /// 字符串格式化操作失败。
    #[error("字符串格式化失败: {0}")]
    Format(#[from] std::fmt::Error),

    /// 表示程序内部逻辑错误或未明确分类的运行时错误。
    #[error("内部逻辑错误: {0}")]
    Internal(String),

    /// 文件读写等输入/输出操作失败。
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),

    /// 字节序列无法安全地转换为 UTF-8 编码的字符串。
    #[error("UTF-8 编码转换错误: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
}

/// 当从字符串解析 `CanonicalMetadataKey` 失败时返回的特定错误类型。
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParseCanonicalMetadataKeyError(pub String); // 存储无法解析的原始键字符串

impl std::fmt::Display for ParseCanonicalMetadataKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "未知或无效的元数据键: {}", self.0)
    }
}
impl std::error::Error for ParseCanonicalMetadataKeyError {}

//=============================================================================
// 2. 核心歌词格式枚举 (Lyric Format Enum)
//=============================================================================

/// 枚举，表示本工具支持的歌词源文件格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, Serialize, Deserialize)]
#[strum(ascii_case_insensitive)]
#[derive(Default)]
pub enum LyricFormat {
    /// Timed Text Markup Language 格式，是 Apple Music 等平台使用的标准格式。
    #[default]
    Ttml,
}

//=============================================================================
// 3. 歌词内部表示结构 (Internal Lyric Representation)
//=============================================================================

/// 代表逐字歌词中的一个基本时间单位——音节。
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricSyllable {
    /// 音节的文本内容 (例如 "你", "好")。
    pub text: String,
    /// 音节开始时间，相对于歌曲开始的绝对时间（单位：毫秒）。
    pub start_ms: u64,
    /// 音节结束时间，相对于歌曲开始的绝对时间（单位：毫秒）。
    pub end_ms: u64,
    /// 可选的音节持续时间（毫秒），通常等于 `end_ms - start_ms`。
    pub duration_ms: Option<u64>,
    /// 指示该音节后面是否紧跟一个空格。
    pub ends_with_space: bool,
}

/// 存储一行歌词的翻译文本及其语言信息。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationEntry {
    /// 翻译的文本内容。
    pub text: String,
    /// 翻译的目标语言代码 (遵循 BCP 47 标准，例如 "zh-Hans", "en")，可选。
    pub lang: Option<String>,
}

/// 存储一行歌词的罗马音（或其他音译）及其语言/方案信息。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RomanizationEntry {
    /// 罗马音的文本内容。
    pub text: String,
    /// 音译的语言 (例如 "ja-Latn-JP-heploc")，可选。
    pub lang: Option<String>,
    /// 可选的特定罗马音方案名称 (例如 "hepburn")。
    pub scheme: Option<String>,
}

/// 代表歌词行中的背景人声或和声部分。
/// 其结构与主歌词行相似，可以包含自己的音节、翻译和罗马音。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundSection {
    /// 背景歌词部分的整体开始时间（毫秒）。
    pub start_ms: u64,
    /// 背景歌词部分的整体结束时间（毫秒）。
    pub end_ms: u64,
    /// 背景部分的音节列表。
    pub syllables: Vec<LyricSyllable>,
    /// 背景部分的翻译列表。
    pub translations: Vec<TranslationEntry>,
    /// 背景部分的罗马音列表。
    pub romanizations: Vec<RomanizationEntry>,
}

/// **核心数据结构**：代表一行完整的歌词。
///
/// 这是整个程序中处理歌词数据的主要单元，它聚合了所有相关信息，
/// 包括时间、文本、音节、翻译、演唱者和歌曲结构等。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricLine {
    /// 行的开始时间（毫秒）。
    pub start_ms: u64,
    /// 行的结束时间（毫秒）。
    pub end_ms: u64,
    /// 可选的整行歌词的纯文本表示。对于逐行歌词是必须的，对于逐字歌词可以由音节重组而成。
    pub line_text: Option<String>,
    /// 主歌词的音节列表，用于实现逐字定时效果。
    pub main_syllables: Vec<LyricSyllable>,
    /// 该行的翻译列表。
    pub translations: Vec<TranslationEntry>,
    /// 该行的罗马音列表。
    pub romanizations: Vec<RomanizationEntry>,
    /// 可选的演唱者或角色标识 (例如，在 TTML 中对应 `ttm:agent` 的 "v1", "v2")。
    pub agent: Option<String>,
    /// 可选的背景歌词部分。
    pub background_section: Option<BackgroundSection>,
    /// 可选的歌曲组成部分标记 (例如 "Verse", "Chorus", "Bridge")。
    pub song_part: Option<String>,
    /// 可选的 iTunes 特定行键 (例如 "L1", "L2")，用于关联元数据中的翻译。
    pub itunes_key: Option<String>,
}

//=============================================================================
// 4. 元数据结构体 (Metadata Structures)
//=============================================================================

/// 定义元数据的规范化键，用于将不同来源的元数据键统一化。
///
/// 例如，"title"、"musicName"、"TITLE" 都会被解析为 `CanonicalMetadataKey::Title`。
/// 这样做的好处是简化了后续处理逻辑，无需再处理多种键的变体。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CanonicalMetadataKey {
    /// 歌曲标题。
    Title,
    /// 艺术家/演唱者。
    Artist,
    /// 专辑名称。
    Album,
    /// 主歌词的语言代码 (遵循 BCP 47 标准)。
    Language,
    /// 全局时间偏移量（毫秒），用于整体调整歌词时间轴。
    Offset,
    /// 词曲作者。
    Songwriter,
    /// 网易云音乐的歌曲 ID。
    NcmMusicId,
    /// QQ音乐的歌曲 ID。
    QqMusicId,
    /// Spotify 的歌曲 ID。
    SpotifyId,
    /// Apple Music 的歌曲 ID。
    AppleMusicId,
    /// 国际标准音像制品编码 (International Standard Recording Code)。
    Isrc,
    /// 逐词歌词作者 GitHub ID。
    TtmlAuthorGithub,
    /// 逐词歌词作者 GitHub 用户名。
    TtmlAuthorGithubLogin,
    /// 所有其他未明确定义的、非标准的或自定义的元数据键。
    Custom(String),
}

impl fmt::Display for CanonicalMetadataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key_name = match self {
            Self::Title => "Title",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::Language => "Language",
            Self::Offset => "Offset",
            Self::Songwriter => "Songwriter",
            Self::NcmMusicId => "NcmMusicId",
            Self::QqMusicId => "QqMusicId",
            Self::SpotifyId => "SpotifyId",
            Self::AppleMusicId => "AppleMusicId",
            Self::Isrc => "Isrc",
            Self::TtmlAuthorGithub => "TtmlAuthorGithub",
            Self::TtmlAuthorGithubLogin => "TtmlAuthorGithubLogin",
            Self::Custom(s) => s.as_str(),
        };
        write!(f, "{}", key_name)
    }
}

impl CanonicalMetadataKey {
    /// 判断一个元数据键是否为“公共”的，通常用于决定是否将其输出。
    ///
    /// 像 `Language` 这样的键更多是用于内部处理，而非展示。
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

/// 实现 `FromStr` trait，使得可以从字符串直接解析为 `CanonicalMetadataKey`。
impl FromStr for CanonicalMetadataKey {
    type Err = ParseCanonicalMetadataKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 将输入字符串转换为小写以实现不区分大小写的匹配
        match s.to_lowercase().as_str() {
            // 为标准键定义别名
            "title" | "musicname" => Ok(Self::Title),
            "artist" | "artists" => Ok(Self::Artist),
            "album" => Ok(Self::Album),
            "language" | "lang" => Ok(Self::Language),
            "offset" => Ok(Self::Offset),
            "songwriter" | "songwriters" => Ok(Self::Songwriter),
            "ncmmusicid" => Ok(Self::NcmMusicId),
            "qqmusicid" => Ok(Self::QqMusicId),
            "spotifyid" => Ok(Self::SpotifyId),
            "applemusicid" => Ok(Self::AppleMusicId),
            "isrc" => Ok(Self::Isrc),
            "ttmlauthorgithub" => Ok(Self::TtmlAuthorGithub),
            "ttmlauthorgithublogin" => Ok(Self::TtmlAuthorGithubLogin),
            // 如果不匹配任何标准键，则视为自定义键
            custom_key if !custom_key.is_empty() => Ok(Self::Custom(custom_key.to_string())),
            // 空字符串被视为无效输入
            _ => Err(ParseCanonicalMetadataKeyError(s.to_string())),
        }
    }
}

//=============================================================================
// 5. 处理流程数据结构 (Processing Flow Structures)
//=============================================================================

/// 存储从源文件解析出的、待进一步处理的完整歌词数据。
///
/// 这是解析器模块 (`ttml_parser.rs`) 的主要输出，也是后续处理步骤的主要输入。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ParsedSourceData {
    /// 解析后的歌词行列表。
    pub lines: Vec<LyricLine>,
    /// 从文件头或特定元数据标签中解析出的原始（未规范化）元数据。
    pub raw_metadata: HashMap<String, Vec<String>>,
    /// 解析的源文件格式。
    pub source_format: LyricFormat,
    /// 可选的原始文件名，用于日志或错误报告。
    pub source_filename: Option<String>,
    /// 指示源文件是否是逐行定时歌词。
    pub is_line_timed_source: bool,
    /// 解析过程中产生的非致命性警告信息列表。
    pub warnings: Vec<String>,
    /// 在解析某些格式（如内嵌TTML的JSON）时，此字段存储原始的TTML字符串内容。
    pub raw_ttml_from_input: Option<String>,
    /// 指示输入的TTML是否被检测为格式化（带缩进）的。
    pub detected_formatted_ttml_input: Option<bool>,
}

//=============================================================================
// 6. TTML 相关选项 (TTML Options)
//=============================================================================

/// TTML 生成时的计时模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TtmlTimingMode {
    /// 逐字计时模式，会为每个音节生成带时间戳的 `<span>`。
    #[default]
    Word,
    /// 逐行计时模式，只为每行歌词 `<p>` 设置时间戳。
    Line,
}

/// 控制 TTML 文件生成行为的选项。
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
    /// 是否输出带缩进和换行的、易于阅读的 TTML 文件。
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

/// TTML 解析时，当某些语言信息缺失时使用的默认值。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefaultLanguageOptions {
    /// 默认的主语言。
    pub main: Option<String>,
    /// 默认的翻译语言。
    pub translation: Option<String>,
    /// 默认的罗马音语言。
    pub romanization: Option<String>,
}

// =============================================================================
// 7. 歌词优化选项 (Lyric Optimization Options)
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
