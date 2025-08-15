use lyrics_helper_core::MetadataStore;
use lyrics_helper_core::{CanonicalMetadataKey, LyricLine};

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

#[cfg(test)]
mod tests {
    use super::*;
    use lyrics_helper_core::{
        AnnotatedTrack, ContentType, LyricLine, LyricLineBuilder, LyricSyllable,
        LyricSyllableBuilder, LyricTrack, MetadataStore, Word,
    };

    fn build_valid_metadata_store() -> MetadataStore {
        let mut store = MetadataStore::new();
        store.set_multiple("musicName", vec!["Test Song".to_string()]);
        store.set_multiple("artists", vec!["Test Artist".to_string()]);
        store.set_multiple("album", vec!["Test Album".to_string()]);
        store.set_multiple("ncmMusicId", vec!["12345".to_string()]);
        store
    }

    fn build_valid_lyric_line() -> LyricLine {
        let syllable = LyricSyllableBuilder::default()
            .text("Hello".to_string())
            .start_ms(0)
            .end_ms(1000)
            .build()
            .unwrap();
        let track = AnnotatedTrack {
            content_type: ContentType::Main,
            content: LyricTrack {
                words: vec![Word {
                    syllables: vec![syllable],
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        LyricLineBuilder::default()
            .start_ms(0)
            .end_ms(1000)
            .track(track)
            .build()
            .unwrap()
    }

    #[test]
    fn test_validation_success() {
        let metadata_store = build_valid_metadata_store();
        let lines = vec![build_valid_lyric_line()];
        assert!(validate_lyrics_and_metadata(&lines, &metadata_store).is_ok());
    }

    #[test]
    fn test_metadata_missing_title() {
        let mut metadata_store = build_valid_metadata_store();
        metadata_store.remove("musicName");
        let result = validate_lyrics_and_metadata(&[], &metadata_store);
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("musicName")));
    }

    #[test]
    fn test_metadata_missing_artist() {
        let mut metadata_store = build_valid_metadata_store();
        metadata_store.remove("artists");
        let result = validate_lyrics_and_metadata(&[], &metadata_store);
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("artists")));
    }

    #[test]
    fn test_metadata_missing_album() {
        let mut metadata_store = build_valid_metadata_store();
        metadata_store.remove("album");
        let result = validate_lyrics_and_metadata(&[], &metadata_store);
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("album")));
    }

    #[test]
    fn test_metadata_missing_platform_ids() {
        let mut metadata_store = MetadataStore::new();
        metadata_store.set_multiple("musicName", vec!["Test Song".to_string()]);
        metadata_store.set_multiple("artists", vec!["Test Artist".to_string()]);
        metadata_store.set_multiple("album", vec!["Test Album".to_string()]);
        let result = validate_lyrics_and_metadata(&[], &metadata_store);
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("任何音乐平台 ID")));
    }

    #[test]
    fn test_lyrics_are_empty() {
        let metadata_store = build_valid_metadata_store();
        let lines = vec![];
        let result = validate_lyrics_and_metadata(&lines, &metadata_store);
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e == "歌词内容为空。"));
    }

    #[test]
    fn test_lyrics_all_timestamps_zero() {
        let metadata_store = build_valid_metadata_store();
        let syllable = LyricSyllable {
            text: "test".to_string(),
            ..Default::default()
        };
        let track = AnnotatedTrack {
            content: LyricTrack {
                words: vec![Word {
                    syllables: vec![syllable],
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let line = LyricLine {
            tracks: vec![track],
            ..Default::default()
        };
        let lines = vec![line];
        let result = validate_lyrics_and_metadata(&lines, &metadata_store);
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e == "所有歌词的时间戳均为 0。"));
    }

    #[test]
    fn test_lyrics_line_has_no_text_content() {
        let metadata_store = build_valid_metadata_store();
        let syllable = LyricSyllableBuilder::default()
            .text("  ".to_string())
            .start_ms(0)
            .end_ms(1000)
            .build()
            .unwrap();
        let track = AnnotatedTrack {
            content: LyricTrack {
                words: vec![Word {
                    syllables: vec![syllable],
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let line = LyricLine {
            start_ms: 0,
            end_ms: 1000,
            tracks: vec![track],
            ..Default::default()
        };

        let result = validate_lyrics_and_metadata(&[line], &metadata_store);
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e == "第 1 行歌词内容为空。"));
    }

    #[test]
    fn test_lyrics_line_end_ms_before_start_ms() {
        let metadata_store = build_valid_metadata_store();
        let mut line = build_valid_lyric_line();
        line.start_ms = 2000;
        line.end_ms = 1000;

        let result = validate_lyrics_and_metadata(&[line], &metadata_store);
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("结束时间 (1000) 小于开始时间 (2000)"))
        );
    }

    #[test]
    fn test_lyrics_syllable_end_ms_before_start_ms() {
        let metadata_store = build_valid_metadata_store();
        let invalid_syllable = LyricSyllableBuilder::default()
            .text("Error".to_string())
            .start_ms(1500)
            .end_ms(1200)
            .build()
            .unwrap();

        let mut line = build_valid_lyric_line();
        line.tracks[0].content.words[0].syllables = vec![invalid_syllable];

        let result = validate_lyrics_and_metadata(&[line], &metadata_store);
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("结束时间 (1200) 小于开始时间 (1500)"));
    }
}
