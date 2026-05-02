use std::fmt::Write as _;

use ttml_processor::model::{Syllable, TTMLResult};

#[derive(Default)]
pub struct LysGenerator;

impl LysGenerator {
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
                line.agent_id.as_deref(),
            );
            output_lines.push(main_line_str);

            if let Some(bg_line) = &line.background_vocal {
                let bg_line_str = self.format_line(
                    bg_line.start_time,
                    bg_line.end_time,
                    &bg_line.text,
                    bg_line.words.as_deref(),
                    true,
                    line.agent_id.as_deref(),
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
        agent_id: Option<&str>,
    ) -> String {
        let current_agent_id = agent_id.unwrap_or("v1");
        let is_duet = current_agent_id != "v1";

        // #### 歌词行属性信息
        // | 属性  | 背景人声 | 对唱视图 |
        // | :---: | :------: | :------: |
        // |   0   |  未设置  |  未设置  |
        // |   1   |  未设置  |    左    |
        // |   2   |  未设置  |    右    |
        // |   3   |    否    |  未设置  |
        // |   4   |    否    |    左    |
        // |   5   |    否    |    右    |
        // |   6   |    是    |  未设置  |
        // |   7   |    是    |    左    |
        // |   8   |    是    |    右    |
        // 在这里写入带有未设置属性的数字非常糟糕，部分下游应用可能会自己推测并应用这些未设置的属性
        // 但为了避免增加几千份文件的改动，这里暂时保持和之前 amll_lyric 的行为一致
        let property_id = match (is_bg, is_duet) {
            (true, true) => 8,
            (true, false) => 6,
            (false, true) => 2,
            (false, false) => 0,
        };

        let line_duration = end_time.saturating_sub(start_time);
        let mut words_str = String::new();

        match words {
            Some(w) if !w.is_empty() => {
                let len = w.len();
                for (i, word) in w.iter().enumerate() {
                    let word_start = word.start_time;
                    let word_duration = word.end_time.saturating_sub(word_start);

                    let mut word_text = word.text.clone();
                    if is_bg {
                        if i == 0 {
                            word_text.insert(0, '(');
                        }
                        if i == len - 1 {
                            word_text.push(')');
                        }
                    }

                    let space = if word.ends_with_space.unwrap_or(false) {
                        " "
                    } else {
                        ""
                    };
                    let _ = write!(
                        words_str,
                        "{word_text}{space}({word_start},{word_duration})"
                    );
                }
            }
            _ => {
                // 逐行文本直接当作一个大音节
                let mut text_to_use = text.to_string();

                if is_bg {
                    text_to_use = format!("({text_to_use})");
                }

                let _ = write!(words_str, "{text_to_use}({start_time},{line_duration})");
            }
        }

        format!("[{property_id}]{words_str}")
    }
}
