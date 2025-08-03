//! 包含一些工具函数的模块。

/// 规范化文本中的空白字符
pub(crate) fn normalize_text_whitespace(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.split_whitespace().collect::<Vec<&str>>().join(" ")
}
