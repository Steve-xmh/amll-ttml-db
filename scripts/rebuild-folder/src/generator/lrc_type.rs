use std::fmt::Write as _;

use ttml_processor::model::{LyricLine, Syllable, TTMLResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LrcMode {
    #[default]
    Plain,
    Enhanced,
    Spl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InlineBracket {
    Angle,
    #[default]
    Square,
}

/// 普通 LRC 结束时间戳的输出策略
/// - None: 不输出
/// - Always: 总是输出
/// - Interval: 间隔大于设定值时输出
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndTimestampMode {
    #[default]
    None,
    Always,
    Interval,
}

/// 辅助行输出顺序
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuxLineOrder {
    #[default]
    TranslationFirst,
    RomanizationFirst,
}

#[derive(Debug, Clone)]
pub struct LrcTypeEndTimestampConfig {
    /// 普通 LRC 结束时间戳的输出策略
    /// - None: 不输出
    /// - Always: 总是输出
    /// - Interval: 间隔大于设定值时输出
    ///
    /// 默认 interval
    pub mode: EndTimestampMode,

    /// 触发间隔（毫秒），仅在 mode 为 "interval" 时有效
    ///
    /// 默认 5000
    pub interval_gap: u32,
}

impl Default for LrcTypeEndTimestampConfig {
    fn default() -> Self {
        Self {
            mode: EndTimestampMode::None,
            interval_gap: 5000,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LrcTypeAuxiliaryLineOptions {
    /// 是否输出该辅助行
    pub enabled: bool,

    /// 是否内联到主歌词行（仅 plain 模式且逐字模式未开启时有效）
    ///
    /// translation、romanization 与 backgroundVocal 中只有最多一个可以内联
    ///
    /// 内联时辅助文本会以半角圆括号 `(text)` 包裹
    pub inline: bool,
}

#[derive(Debug, Clone)]
pub struct LrcTypeAuxiliaryLinesConfig {
    pub order: AuxLineOrder,
    pub translation: LrcTypeAuxiliaryLineOptions,
    pub romanization: LrcTypeAuxiliaryLineOptions,
    pub background_vocal: LrcTypeAuxiliaryLineOptions,
}

impl Default for LrcTypeAuxiliaryLinesConfig {
    fn default() -> Self {
        Self {
            order: AuxLineOrder::default(),
            translation: LrcTypeAuxiliaryLineOptions::default(),
            romanization: LrcTypeAuxiliaryLineOptions::default(),
            background_vocal: LrcTypeAuxiliaryLineOptions {
                enabled: true,
                inline: false,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct LrcTypeGeneratorOptions {
    /// 生成模式
    /// - Plain: 普通 LRC，仅生成行时间戳，忽略逐字信息
    /// - Enhanced: 增强型 LRC
    /// - Spl: Salt Player Lyrics 格式
    // 目前的实现增强型 LRC 和 SPL 的生成行为是一致的，因为 SPL 格式兼容增强型 LRC
    pub mode: LrcMode,

    /// 逐字时间戳括号类型（`enhanced` 和 `spl` 模式有效）
    /// - Angle: 使用 `<mm:ss.ms>`
    /// - Square: 使用 `[mm:ss.ms]`
    pub inline_bracket: InlineBracket,

    /// 辅助行的输出配置（包括翻译、音译、背景人声）
    pub auxiliary_lines: LrcTypeAuxiliaryLinesConfig,

    /// 普通 LRC 的显式结束时间戳（空行）输出配置
    pub end_timestamp: LrcTypeEndTimestampConfig,

    /// 是否在逐字模式下省略行首的时间戳
    pub skip_line_timestamp: bool,
}

impl Default for LrcTypeGeneratorOptions {
    fn default() -> Self {
        Self {
            mode: LrcMode::default(),
            inline_bracket: InlineBracket::default(),
            auxiliary_lines: LrcTypeAuxiliaryLinesConfig::default(),
            end_timestamp: LrcTypeEndTimestampConfig::default(),
            skip_line_timestamp: true,
        }
    }
}

/// LRC 歌词生成器
pub struct LrcTypeGenerator {
    options: LrcTypeGeneratorOptions,
}

impl LrcTypeGenerator {
    pub const fn new(options: LrcTypeGeneratorOptions) -> Self {
        Self { options }
    }

    pub fn generate(&self, ir: &TTMLResult) -> String {
        let mut output_lines = Vec::new();

        let mut iter = ir.lines.iter().peekable();
        while let Some(line) = iter.next() {
            let next_line = iter.peek().copied();
            self.process_single_line(line, next_line, &mut output_lines);
        }

        output_lines.join("\n")
    }

    fn process_single_line(
        &self,
        line: &LyricLine,
        next_line: Option<&LyricLine>,
        output_lines: &mut Vec<String>,
    ) {
        let aux_config = &self.options.auxiliary_lines;
        let mode = self.options.mode;

        let output_translations = aux_config.translation.enabled;
        let output_romanizations = aux_config.romanization.enabled;
        let output_background_vocals = aux_config.background_vocal.enabled;

        let trans_inline = mode == LrcMode::Plain && aux_config.translation.inline;
        let roma_inline = mode == LrcMode::Plain && !trans_inline && aux_config.romanization.inline;
        let bgv_inline = mode == LrcMode::Plain
            && !trans_inline
            && !roma_inline
            && aux_config.background_vocal.inline;

        let line_time_tag = self.format_time(line.start_time, InlineBracket::Square);

        let mut trans_texts: Vec<String> = if output_translations {
            line.translations
                .as_ref()
                .map(|ts| ts.iter().map(|t| t.text.clone()).collect())
                .unwrap_or_default()
        } else {
            vec![]
        };

        let mut roma_texts: Vec<String> = if output_romanizations {
            line.romanizations
                .as_ref()
                .map(|rs| rs.iter().map(|r| r.text.clone()).collect())
                .unwrap_or_default()
        } else {
            vec![]
        };

        let mut bg_line_opt = if output_background_vocals {
            line.background_vocal.clone()
        } else {
            None
        };

        let mut main_line_text = line.text.clone();

        // 辅助行内联处理
        if mode == LrcMode::Plain {
            if trans_inline && !trans_texts.is_empty() {
                let inline_content = trans_texts.remove(0);
                Self::append_inline_text(&mut main_line_text, &inline_content);
            } else if roma_inline && !roma_texts.is_empty() {
                let inline_content = roma_texts.remove(0);
                Self::append_inline_text(&mut main_line_text, &inline_content);
            } else if bgv_inline && let Some(bgv_to_inline) = bg_line_opt.take() {
                Self::append_inline_text(&mut main_line_text, &bgv_to_inline.text);

                if output_translations && let Some(ts) = bgv_to_inline.translations {
                    for (i, t) in ts.into_iter().enumerate() {
                        if i < trans_texts.len() {
                            let _ = write!(trans_texts[i], " ({})", t.text);
                        } else {
                            trans_texts.push(format!("({})", t.text));
                        }
                    }
                }

                if output_romanizations && let Some(rs) = bgv_to_inline.romanizations {
                    for (i, r) in rs.into_iter().enumerate() {
                        if i < roma_texts.len() {
                            let _ = write!(roma_texts[i], " ({})", r.text);
                        } else {
                            roma_texts.push(format!("({})", r.text));
                        }
                    }
                }
            }
        }

        // 渲染主歌词
        output_lines.push(self.render_base_item(
            line.start_time,
            line.end_time,
            &line.text,
            line.words.as_ref(),
            Some(&main_line_text),
            false,
        ));

        // 渲染背景人声
        if let Some(bg_line) = bg_line_opt {
            output_lines.push(self.render_base_item(
                bg_line.start_time,
                bg_line.end_time,
                &bg_line.text,
                bg_line.words.as_ref(),
                None,
                true,
            ));
        }

        // 渲染外置的翻译和音译行
        let ordered_aux_texts = match aux_config.order {
            AuxLineOrder::TranslationFirst => [trans_texts, roma_texts].concat(),
            AuxLineOrder::RomanizationFirst => [roma_texts, trans_texts].concat(),
        };

        for aux_text in ordered_aux_texts {
            output_lines.push(format!("{line_time_tag}{aux_text}"));
        }

        // 显式结束时间戳处理
        self.process_end_timestamp(line, next_line, output_lines);
    }

    fn append_inline_text(target: &mut String, inline_content: &str) {
        if target.is_empty() {
            let _ = write!(target, "({inline_content})");
        } else {
            let _ = write!(target, " ({inline_content})");
        }
    }

    fn render_base_item(
        &self,
        start_time: u32,
        end_time: u32,
        original_text: &str,
        words_opt: Option<&Vec<Syllable>>,
        override_text: Option<&str>,
        is_bgv: bool,
    ) -> String {
        let mode = self.options.mode;
        let time_tag = self.format_time(start_time, InlineBracket::Square);

        let has_words = words_opt.is_some_and(|w| !w.is_empty());
        let should_render_words = (mode == LrcMode::Enhanced || mode == LrcMode::Spl) && has_words;

        let mut output = String::new();

        if should_render_words {
            // 逐字模式
            let words = words_opt.unwrap();

            if !self.options.skip_line_timestamp {
                output.push_str(&time_tag);
            }

            for (i, word) in words.iter().enumerate() {
                output.push_str(&self.format_time(word.start_time, self.options.inline_bracket));

                let mut word_text = word.text.clone();
                if is_bgv {
                    if i == 0 {
                        word_text.insert(0, '(');
                    }
                    if i == words.len() - 1 {
                        word_text.push(')');
                    }
                }
                output.push_str(&word_text);

                if word.ends_with_space.unwrap_or(false) {
                    output.push(' ');
                }

                // 最后一个词追加结束时间
                if i == words.len() - 1 {
                    output.push_str(&self.format_time(word.end_time, self.options.inline_bracket));
                }
            }
        } else {
            // 逐行模式或 fallback
            let mut text_to_use = override_text.unwrap_or(original_text).to_string();
            if is_bgv {
                text_to_use = format!("({text_to_use})");
            }
            output.push_str(&time_tag);
            output.push_str(&text_to_use);

            if mode == LrcMode::Enhanced || mode == LrcMode::Spl {
                output.push_str(&self.format_time(end_time, InlineBracket::Square));
            }
        }

        output
    }

    fn process_end_timestamp(
        &self,
        line: &LyricLine,
        next_line: Option<&LyricLine>,
        output_lines: &mut Vec<String>,
    ) {
        let end_ts = &self.options.end_timestamp;

        if self.options.mode == LrcMode::Plain && end_ts.mode != EndTimestampMode::None {
            let gap = next_line.map_or(u32::MAX, |next| {
                next.start_time.saturating_sub(line.end_time)
            });

            if end_ts.mode == EndTimestampMode::Always
                || (end_ts.mode == EndTimestampMode::Interval && gap >= end_ts.interval_gap)
            {
                output_lines.push(self.format_time(line.end_time, InlineBracket::Square));
            }
        }
    }

    fn format_time(&self, ms: u32, bracket: InlineBracket) -> String {
        let total_sec = ms / 1000;
        let min = total_sec / 60;
        let sec = total_sec % 60;
        let milli = ms % 1000;

        let content = format!("{min:02}:{sec:02}.{milli:03}");
        match bracket {
            InlineBracket::Angle => format!("<{content}>"),
            InlineBracket::Square => format!("[{content}]"),
        }
    }
}
