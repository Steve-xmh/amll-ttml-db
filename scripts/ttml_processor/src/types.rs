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
    Format(#[from] std::fmt::Error),
    /// 逻辑错误或未明确分类的错误。
    #[error("错误: {0}")]
    Internal(String),
    /// 文件读写等IO错误。
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),
    /// 从字节序列转换为 UTF-8 字符串失败。
    #[error("UTF-8 转换错误: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
}

/// 定义从字符串解析 `CanonicalMetadataKey` 时可能发生的错误。
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParseCanonicalMetadataKeyError(String); // 存储无法解析的原始键字符串

impl std::fmt::Display for ParseCanonicalMetadataKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "未知或无效的元数据键: {}", self.0)
    }
}
impl std::error::Error for ParseCanonicalMetadataKeyError {}

//=============================================================================
// 2. 核心歌词格式枚举及相关
//=============================================================================

/// 枚举：表示支持的歌词格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, Serialize, Deserialize)]
#[strum(ascii_case_insensitive)]
#[derive(Default)]
pub enum LyricFormat {
    /// Timed Text Markup Language 格式。
    #[default]
    Ttml,
}

//=============================================================================
// 3. 歌词内部表示结构
//=============================================================================

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
    pub duration_ms: Option<u64>,
    /// 指示该音节后是否应有空格。
    pub ends_with_space: bool,
}

/// 表示单个翻译及其语言的结构体。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationEntry {
    /// 翻译的文本内容。
    pub text: String,
    /// 翻译的语言代码，可选。
    pub lang: Option<String>,
}

/// 表示单个罗马音及其语言/方案的结构体。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RomanizationEntry {
    /// 罗马音的文本内容。
    pub text: String,
    /// 目标音译的语言和脚本代码，可选。
    pub lang: Option<String>,
    /// 可选的特定罗马音方案名称。
    pub scheme: Option<String>,
}

/// 表示歌词行中的背景歌词部分。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundSection {
    /// 背景歌词的开始时间（毫秒）。
    pub start_ms: u64,
    /// 背景歌词的结束时间（毫秒）。
    pub end_ms: u64,
    /// 背景歌词的音节列表。
    pub syllables: Vec<LyricSyllable>,
    /// 背景歌词的翻译。
    pub translations: Vec<TranslationEntry>,
    /// 背景歌词的罗马音。
    pub romanizations: Vec<RomanizationEntry>,
}

/// 通用的歌词行结构，作为项目内部处理歌词数据的主要表示。
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricLine {
    /// 行的开始时间（毫秒）。
    pub start_ms: u64,
    /// 行的结束时间（毫秒）。
    pub end_ms: u64,
    /// 可选的整行文本内容。
    pub line_text: Option<String>,
    /// 主歌词的音节列表。
    pub main_syllables: Vec<LyricSyllable>,
    /// 该行的翻译列表。
    pub translations: Vec<TranslationEntry>,
    /// 该行的罗马音列表。
    pub romanizations: Vec<RomanizationEntry>,
    /// 可选的演唱者或角色标识。
    pub agent: Option<String>,
    /// 可选的背景歌词部分。
    pub background_section: Option<BackgroundSection>,
    /// 可选的歌曲组成部分标记。
    pub song_part: Option<String>,
    /// 可选的 iTunes Key (如 "L1", "L2")。
    pub itunes_key: Option<String>,
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
    /// TTML歌词贡献者的GitHub ID。
    TtmlAuthorGithub,
    /// TTML歌词贡献者的GitHub登录名。
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
        write!(f, "{}", key_name)
    }
}

impl CanonicalMetadataKey {
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
            custom_key if !custom_key.is_empty() => Ok(Self::Custom(custom_key.to_string())),
            _ => Err(ParseCanonicalMetadataKeyError(s.to_string())),
        }
    }
}

//=============================================================================
// 5. 处理与数据结构体
//=============================================================================

/// 存储从源文件解析出的、准备进行进一步处理或转换的歌词数据。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ParsedSourceData {
    /// 解析后的歌词行列表。
    pub lines: Vec<LyricLine>,
    /// 从文件头或特定元数据标签中解析出的原始（未规范化）元数据。
    pub raw_metadata: HashMap<String, Vec<String>>,
    /// 解析的源文件格式。
    pub source_format: LyricFormat,
    /// 可选的原始文件名。
    pub source_filename: Option<String>,
    /// 指示源文件是否是逐行歌词。
    pub is_line_timed_source: bool,
    /// 解析过程中产生的警告信息列表。
    pub warnings: Vec<String>,
    /// 如果源文件是内嵌TTML的JSON，此字段存储原始的TTML字符串内容。
    pub raw_ttml_from_input: Option<String>,
    /// 指示输入的TTML是否被格式化。
    pub detected_formatted_ttml_input: Option<bool>,
}

//=============================================================================
// 6. TTML 相关选项
//=============================================================================

/// TTML 生成时的计时模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TtmlTimingMode {
    #[default]
    Word, // 逐字计时
    Line, // 逐行计时
}

/// TTML 生成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtmlGenerationOptions {
    pub timing_mode: TtmlTimingMode,
    pub main_language: Option<String>,
    pub translation_language: Option<String>,
    pub romanization_language: Option<String>,
    pub use_apple_format_rules: bool,
    /// 是否输出格式化后的 TTML
    pub format: bool,
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
        }
    }
}

/// TTML 解析时使用的默认语言选项
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefaultLanguageOptions {
    pub main: Option<String>,
    pub translation: Option<String>,
    pub romanization: Option<String>,
}
