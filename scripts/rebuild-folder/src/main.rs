use std::{io::BufReader, path::Path};

use amll_lyric::ttml::TTMLLyric;
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let t = std::time::Instant::now();
    let cwd = std::env::current_dir().unwrap();
    let root_dir = cwd.join("../../");
    let raw_dir = root_dir.join("raw-lyrics");
    let old_ncm_dir = root_dir.join("lyrics");
    let ncm_dir = root_dir.join("ncm-lyrics");
    let spotify_dir = root_dir.join("spotify-lyrics");
    let qq_dir = root_dir.join("qq-lyrics");
    let am_dir = root_dir.join("am-lyrics");
    let _ = std::fs::remove_dir_all(&ncm_dir);
    let _ = std::fs::remove_dir_all(&old_ncm_dir);
    let _ = std::fs::remove_dir_all(&spotify_dir);
    let _ = std::fs::remove_dir_all(&qq_dir);
    let _ = std::fs::remove_dir_all(&am_dir);
    std::fs::create_dir_all(&ncm_dir)?;
    std::fs::create_dir_all(&old_ncm_dir)?;
    std::fs::create_dir_all(&spotify_dir)?;
    std::fs::create_dir_all(&qq_dir)?;
    std::fs::create_dir_all(&am_dir)?;
    let mut raw_lyrics = std::fs::read_dir(raw_dir)
        .expect("无法打开 raw-lyrics 文件夹")
        .flatten()
        .collect::<Vec<_>>();
    raw_lyrics.sort_by_key(|x| {
        let p = x.file_name();
        let s = p.to_string_lossy();
        let s = s.split('-').next().unwrap_or_default();
        s.parse::<u64>().expect("无法解析提交时间戳")
    });
    let generate_lyric_files = |lyric: &TTMLLyric, dest: &Path, name: &str| -> anyhow::Result<()> {
        let file_path = dest.join(name).with_extension("ttml");
        std::fs::write(file_path, amll_lyric::ttml::stringify_ttml(lyric)?)?;
        let file_path = dest.join(name).with_extension("lrc");
        std::fs::write(file_path, amll_lyric::lrc::stringify_lrc(&lyric.lines))?;
        let file_path = dest.join(name).with_extension("yrc");
        std::fs::write(file_path, amll_lyric::yrc::stringify_yrc(&lyric.lines))?;
        let file_path = dest.join(name).with_extension("lys");
        std::fs::write(file_path, amll_lyric::lys::stringify_lys(&lyric.lines))?;
        let file_path = dest.join(name).with_extension("qrc");
        std::fs::write(file_path, amll_lyric::qrc::stringify_qrc(&lyric.lines))?;
        let file_path = dest.join(name).with_extension("eslrc");
        std::fs::write(file_path, amll_lyric::eslrc::stringify_eslrc(&lyric.lines))?;
        Ok(())
    };
    println!("正在构建所有歌词文件夹，总计 {} 个歌词文件", raw_lyrics.len());
    for entry in raw_lyrics {
        let file = std::fs::File::open(entry.path())
            .with_context(|| format!("无法打开歌词文件 {:?}", entry.file_name()))?;
        let lyric_data = amll_lyric::ttml::parse_ttml(BufReader::new(file))
            .with_context(|| format!("解析歌词文件 {:?} 时出错", entry.file_name()))?;
        for (key, values) in lyric_data.metadata.iter() {
            match key.as_ref() {
                "ncmMusicId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &ncm_dir, id.as_ref())?;
                        generate_lyric_files(&lyric_data, &old_ncm_dir, id.as_ref())?;
                    }
                }
                "spotifyId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &spotify_dir, id.as_ref())?;
                    }
                }
                "qqMusicId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &qq_dir, id.as_ref())?;
                    }
                }
                "appleMusicId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &am_dir, id.as_ref())?;
                    }
                }
                _ => {}
            }
        }
        // println!("文件: {}", file.file_name().to_string_lossy());
    }
    println!("构建完成，耗时: {:?}", t.elapsed());

    Ok(())
}
