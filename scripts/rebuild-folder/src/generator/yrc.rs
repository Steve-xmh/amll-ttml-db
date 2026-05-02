use std::fmt::Write as _;

use ttml_processor::model::{Syllable, TTMLResult};

#[derive(Default)]
pub struct YrcGenerator;

impl YrcGenerator {
    pub const fn new() -> Self {
        Self
    }

    pub fn generate(&self, ir: &TTMLResult) -> String {
        let mut output_lines = Vec::new();

        for line in &ir.lines {
            let main_line_str = self.format_line(
                line.start_time,
                line.end_time,
                &line.text,
                line.words.as_deref(),
                false,
            );
            output_lines.push(main_line_str);

            if let Some(bg_line) = &line.background_vocal {
                let bg_line_str = self.format_line(
                    bg_line.start_time,
                    bg_line.end_time,
                    &bg_line.text,
                    bg_line.words.as_deref(),
                    true,
                );
                output_lines.push(bg_line_str);
            }
        }

        output_lines.join("\n")
    }

    fn format_line(
        &self,
        start_time: u32,
        end_time: u32,
        text: &str,
        words: Option<&[Syllable]>,
        is_bg: bool,
    ) -> String {
        let line_duration = end_time.saturating_sub(start_time);
        let mut words_str = String::new();

        match words {
            Some(w) if !w.is_empty() => {
                let len = w.len();
                for (i, word) in w.iter().enumerate() {
                    let word_start = word.start_time;
                    let word_duration = word.end_time.saturating_sub(word_start);

                    // 目前已知 YRC 不允许直接出现英文括号，所以要换成中文括号
                    let mut safe_text = word.text.replace('(', "（").replace(')', "）");

                    if is_bg {
                        if i == 0 {
                            safe_text.insert(0, '（');
                        }
                        if i == len - 1 {
                            safe_text.push('）');
                        }
                    }

                    let space = if word.ends_with_space.unwrap_or(false) {
                        " "
                    } else {
                        ""
                    };
                    let _ = write!(
                        words_str,
                        "({word_start},{word_duration},0){safe_text}{space}"
                    );
                }
            }
            _ => {
                // 逐行文本直接当作一个大音节
                let mut safe_text = text.replace('(', "（").replace(')', "）");

                if is_bg {
                    safe_text.insert(0, '（');
                    safe_text.push('）');
                }

                let _ = write!(words_str, "({start_time},{line_duration},0){safe_text}");
            }
        }

        format!("[{start_time},{line_duration}]{words_str}")
    }
}
