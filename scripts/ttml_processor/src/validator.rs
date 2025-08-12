use crate::metadata_processor::MetadataStore;
use crate::types::{CanonicalMetadataKey, LyricLine};

/// 对歌词数据和元数据进行验证。
///
/// # 参数
///
/// * `lines` - 一个 `LyricLine` 结构体的切片，代表所有歌词行。
/// * `metadata_store` - 一个 `MetadataStore` 的引用，包含所有解析出的元数据。
///
/// # 返回
///
/// * `Ok(())` - 如果所有验证均通过。
/// * `Err(Vec<String>)` - 如果发现任何问题。
pub fn validate_lyrics_and_metadata(
    lines: &[LyricLine],
    metadata_store: &MetadataStore,
) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();

    validate_metadata(metadata_store, &mut errors);

    validate_lyric_lines(lines, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// 验证元数据的完整性。
fn validate_metadata(metadata_store: &MetadataStore, errors: &mut Vec<String>) {
    if metadata_store
        .get_multiple_values(&CanonicalMetadataKey::Title)
        .is_none()
    {
        errors.push("歌词文件中未包含歌曲名称信息 (缺失 musicName 元数据)。".to_string());
    }
    if metadata_store
        .get_multiple_values(&CanonicalMetadataKey::Artist)
        .is_none()
    {
        errors.push("歌词文件中未包含音乐作者信息 (缺失 artists 元数据)。".to_string());
    }
    if metadata_store
        .get_multiple_values(&CanonicalMetadataKey::Album)
        .is_none()
    {
        errors.push(
            "歌词文件中未包含专辑信息 (缺失 album 元数据)。(注：如果是单曲专辑请和歌曲名称同名)"
                .to_string(),
        );
    }

    let platform_ids_present = [
        CanonicalMetadataKey::AppleMusicId,
        CanonicalMetadataKey::NcmMusicId,
        CanonicalMetadataKey::SpotifyId,
        CanonicalMetadataKey::QqMusicId,
    ]
    .iter()
    .any(|key| metadata_store.get_multiple_values(key).is_some());

    if !platform_ids_present {
        errors.push("歌词文件中未包含任何音乐平台 ID。".to_string());
    }
}

/// 验证歌词行的内容和时间戳。
fn validate_lyric_lines(lines: &[LyricLine], errors: &mut Vec<String>) {
    if lines.is_empty() {
        errors.push("歌词内容为空。".to_string());
        return;
    }

    // 检查是否所有的时间戳都是 0
    let has_any_non_zero_timestamp = lines.iter().any(|line| {
        line.start_ms != 0
            || line.end_ms != 0
            || line.tracks.iter().any(|track| {
                track.content.words.iter().any(|word| {
                    word.syllables
                        .iter()
                        .any(|s| s.start_ms != 0 || s.end_ms != 0)
                })
            })
    });

    if !has_any_non_zero_timestamp {
        errors.push("所有歌词的时间戳均为 0。".to_string());
    }

    for (line_idx, line) in lines.iter().enumerate() {
        // 检查该行是否有实际文本内容
        let has_content = line.tracks.iter().any(|track| {
            track.content.words.iter().any(|word| {
                word.syllables
                    .iter()
                    .any(|syllable| !syllable.text.trim().is_empty())
            })
        });

        if !has_content {
            errors.push(format!("第 {} 行歌词内容为空。", line_idx + 1));
            continue;
        }

        // 检查行时间戳
        if line.end_ms < line.start_ms {
            errors.push(format!(
                "第 {} 行歌词结束时间 ({}) 小于开始时间 ({}).",
                line_idx + 1,
                line.end_ms,
                line.start_ms
            ));
        }

        // 检查每个轨道中的音节时间戳
        for (track_idx, track) in line.tracks.iter().enumerate() {
            for (word_idx, word) in track.content.words.iter().enumerate() {
                for (syl_idx, syllable) in word.syllables.iter().enumerate() {
                    if syllable.text.trim().is_empty() {
                        continue;
                    }

                    if syllable.end_ms < syllable.start_ms {
                        errors.push(format!(
                            "第 {} 行第 {} 个轨道第 {} 个词第 {} 个音节 '{}' 结束时间 ({}) 小于开始时间 ({}).",
                            line_idx + 1,
                            track_idx + 1,
                            word_idx + 1,
                            syl_idx + 1,
                            syllable.text,
                            syllable.end_ms,
                            syllable.start_ms
                        ));
                    }
                }
            }
        }
    }
}
