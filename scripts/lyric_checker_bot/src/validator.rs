use ttml_processor::model::{LyricLine, TTMLMetadata, TTMLResult};

pub fn validate_lyrics_and_metadata(result: &TTMLResult) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();

    validate_metadata(&result.metadata, &mut errors);

    validate_lyric_lines(&result.lines, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_metadata(meta: &TTMLMetadata, errors: &mut Vec<String>) {
    if meta.title.is_none() {
        errors.push("歌词文件中未包含歌曲名称信息 (缺失 musicName 元数据)".to_string());
    }
    if meta.artist.is_none() {
        errors.push("歌词文件中未包含音乐作者信息 (缺失 artists 元数据)".to_string());
    }
    if meta.album.is_none() {
        errors.push(
            "歌词文件中未包含专辑信息 (缺失 album 元数据) (注：如果是单曲专辑请和歌曲名称同名)"
                .to_string(),
        );
    }

    let has_platform_id = meta
        .platform_ids
        .as_ref()
        .is_some_and(|map| !map.is_empty());

    if !has_platform_id {
        errors.push("歌词文件中未包含任何音乐平台 ID".to_string());
    }
}

fn validate_lyric_lines(lines: &[LyricLine], errors: &mut Vec<String>) {
    if lines.is_empty() {
        errors.push("歌词内容为空".to_string());
        return;
    }

    if !has_any_non_zero_timestamp(lines) {
        errors.push("所有歌词的时间戳均为 0".to_string());
    }

    for (line_idx, line) in lines.iter().enumerate() {
        validate_single_line(line_idx, line, errors);
    }
}

fn has_any_non_zero_timestamp(lines: &[LyricLine]) -> bool {
    lines.iter().any(|line| {
        line.start_time != 0
            || line.end_time != 0
            || line
                .words
                .iter()
                .flatten()
                .any(|s| s.start_time != 0 || s.end_time != 0)
    })
}

fn validate_single_line(line_idx: usize, line: &LyricLine, errors: &mut Vec<String>) {
    let has_content = line
        .words
        .iter()
        .flatten()
        .any(|s| !s.text.trim().is_empty())
        || !line.text.trim().is_empty();

    if !has_content {
        errors.push(format!("第 {} 行歌词内容为空", line_idx + 1));
        return;
    }

    if line.end_time < line.start_time {
        errors.push(format!(
            "第 {} 行歌词结束时间 ({}) 小于开始时间 ({})",
            line_idx + 1,
            line.end_time,
            line.start_time
        ));
    }

    let mut prev_syl_end: Option<u32> = None;

    for (syl_idx, syllable) in line.words.iter().flatten().enumerate() {
        if syllable.text.trim().is_empty() {
            continue;
        }

        if syllable.end_time < syllable.start_time {
            errors.push(format!(
                "第 {} 行第 {} 个音节 '{}' 结束时间 ({}) 小于开始时间 ({})",
                line_idx + 1,
                syl_idx + 1,
                syllable.text,
                syllable.end_time,
                syllable.start_time
            ));
        }

        if let Some(prev_end) = prev_syl_end
            && syllable.start_time < prev_end
        {
            errors.push(format!(
                "第 {} 行第 {} 个音节 '{}' 的开始时间 ({}) 出现倒流 (早于前一个音节的结束时间 {})",
                line_idx + 1,
                syl_idx + 1,
                syllable.text,
                syllable.start_time,
                prev_end
            ));
        }

        let valid_syl_end = std::cmp::max(syllable.start_time, syllable.end_time);
        prev_syl_end = Some(std::cmp::max(prev_syl_end.unwrap_or(0), valid_syl_end));
    }
}
