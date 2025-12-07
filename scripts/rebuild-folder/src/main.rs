use std::{
    borrow::Cow,
    collections::HashMap,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result};
use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use lyrics_helper_core::{DefaultLanguageOptions, TtmlParsingOptions};
use rayon::prelude::*;
use ttml_processor::parse_ttml;

struct ParsedLyric {
    lines: Vec<amll_lyric::LyricLine<'static>>,
    metadata: Vec<(String, Vec<String>)>,
}

struct ParsedEntry {
    path: PathBuf,
    file_name: String,
    data: ParsedLyric,
}

struct ProjectLayout {
    root: PathBuf,
    raw_dir: PathBuf,
    ncm_dir: PathBuf,
    spotify_dir: PathBuf,
    qq_dir: PathBuf,
    am_dir: PathBuf,
    metadata_dir: PathBuf,
}

impl ProjectLayout {
    fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let root_dir = cwd.join("../../");
        Ok(Self {
            raw_dir: root_dir.join("raw-lyrics"),
            ncm_dir: root_dir.join("ncm-lyrics"),
            spotify_dir: root_dir.join("spotify-lyrics"),
            qq_dir: root_dir.join("qq-lyrics"),
            am_dir: root_dir.join("am-lyrics"),
            metadata_dir: root_dir.join("metadata"),
            root: root_dir,
        })
    }

    fn init_directories(&self, gen_folder: bool) -> Result<()> {
        let mut dirs_to_clean = Vec::new();

        if gen_folder {
            dirs_to_clean.push(&self.ncm_dir);
            dirs_to_clean.push(&self.spotify_dir);
            dirs_to_clean.push(&self.qq_dir);
            dirs_to_clean.push(&self.am_dir);
        }
        dirs_to_clean.push(&self.metadata_dir);

        println!("正在重建 {} 个目录...", dirs_to_clean.len());

        dirs_to_clean.par_iter().try_for_each(|dir| -> Result<()> {
            let start = Instant::now();
            let dir_name = dir.file_name().unwrap_or_default().to_string_lossy();

            if dir.exists() {
                std::fs::remove_dir_all(dir)
                    .with_context(|| format!("无法删除旧目录: {:?}", dir.display()))?;
            }
            std::fs::create_dir_all(dir)
                .with_context(|| format!("无法创建新目录: {:?}", dir.display()))?;

            let duration = start.elapsed();
            println!("目录 {dir_name} 重建完毕 耗时 {duration:.2?}");

            Ok(())
        })?;

        Ok(())
    }
}

#[derive(Debug)]
struct Contributor<'a> {
    github_id: Cow<'a, str>,
    count: usize,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
enum Platform {
    Ncm,
    Spotify,
    Qq,
    Am,
}

fn is_git_worktree_clean() -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("无法执行 git status 命令")?;
    Ok(output.stdout.is_empty() && output.stderr.is_empty())
}

fn add_file_to_git(file: &str) -> Result<()> {
    let result = std::process::Command::new("git")
        .args(["add", file])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("无法执行 git add 命令")?;
    anyhow::ensure!(result.success(), "git add 命令执行失败");
    Ok(())
}

fn commit(message: &str) -> Result<()> {
    let result = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("无法执行 git commit 命令")?;
    anyhow::ensure!(result.success(), "git commit 命令执行失败");
    Ok(())
}

fn push(branch: &str) -> Result<()> {
    let result = std::process::Command::new("git")
        .args(["push", "--set-upstream", "origin", branch])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("无法执行 git push 命令")?;
    anyhow::ensure!(result.success(), "git push 命令执行失败");
    Ok(())
}

fn load_raw_lyrics(raw_dir: &Path) -> Result<Vec<std::fs::DirEntry>> {
    let raw_entries = std::fs::read_dir(raw_dir).context("无法打开 raw-lyrics 文件夹")?;

    let mut valid_lyrics: Vec<(u64, std::fs::DirEntry)> = raw_entries
        .flatten()
        .filter_map(|entry| {
            let file_name_os = entry.file_name();
            let file_name = file_name_os.to_string_lossy();

            file_name
                .split('-')
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .map_or_else(
                    || {
                        eprintln!("意外的文件名: {file_name:?}");
                        None
                    },
                    |id| Some((id, entry)),
                )
        })
        .collect();

    valid_lyrics.sort_by_key(|(id, _)| *id);
    let sorted_entries = valid_lyrics.into_iter().map(|(_, entry)| entry).collect();
    Ok(sorted_entries)
}

fn process_lyric_content(file_content: &str) -> Result<ParsedLyric> {
    let parse_opts = TtmlParsingOptions {
        force_timing_mode: None,
        default_languages: DefaultLanguageOptions::default(),
    };

    let parsed_source_data = parse_ttml(file_content, &parse_opts)?;
    let mut lines = Vec::new();

    for new_line in parsed_source_data.lines {
        // agent 为 None 或 v1，视为非对唱，其他情况视为对唱
        let is_duet = !matches!(new_line.agent.as_deref(), Some("v1") | None);
        let mut process_and_push_track = |track: &lyrics_helper_core::AnnotatedTrack,
                                          is_bg: bool| {
            let mut words = Vec::new();
            for syl in track.content.syllables() {
                words.push(amll_lyric::LyricWord {
                    start_time: syl.start_ms,
                    end_time: syl.end_ms,
                    word: Cow::Owned(syl.text.clone()),
                });

                // AMLL 的历史遗留问题，用时间戳均为0的音节表示空格
                if syl.ends_with_space {
                    words.push(amll_lyric::LyricWord {
                        start_time: 0,
                        end_time: 0,
                        word: " ".into(),
                    });
                }
            }

            lines.push(amll_lyric::LyricLine {
                words,
                translated_lyric: Cow::Owned(String::new()),
                roman_lyric: Cow::Owned(String::new()),
                is_bg,
                is_duet,
                start_time: new_line.start_ms,
                end_time: new_line.end_ms,
            });
        };

        if let Some(track) = new_line.main_track() {
            process_and_push_track(track, false);
        }

        if let Some(track) = new_line.background_track() {
            process_and_push_track(track, true);
        }
    }

    let mut metadata = Vec::new();
    for (k, v) in parsed_source_data.raw_metadata {
        metadata.push((k, v));
    }

    metadata.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(ParsedLyric { lines, metadata })
}

fn save_lyric_files_to_disk(
    lines: &[amll_lyric::LyricLine],
    raw_lyric_path: &Path,
    dest_dir: &Path,
    id_name: &str,
) -> Result<()> {
    let base_path = dest_dir.join(id_name);
    std::fs::copy(raw_lyric_path, base_path.with_extension("ttml"))?;
    std::fs::write(
        base_path.with_extension("lrc"),
        amll_lyric::lrc::stringify_lrc(lines),
    )?;
    std::fs::write(
        base_path.with_extension("yrc"),
        amll_lyric::yrc::stringify_yrc(lines),
    )?;
    std::fs::write(
        base_path.with_extension("lys"),
        amll_lyric::lys::stringify_lys(lines),
    )?;
    std::fs::write(
        base_path.with_extension("qrc"),
        amll_lyric::qrc::stringify_qrc(lines),
    )?;
    std::fs::write(
        base_path.with_extension("eslrc"),
        amll_lyric::eslrc::stringify_eslrc(lines),
    )?;
    Ok(())
}

fn generate_contributor_report(
    layout: &ProjectLayout,
    contribution_map: HashMap<Cow<str>, Contributor>,
) -> Result<()> {
    let mut contribution_list = contribution_map.into_iter().collect::<Vec<_>>();
    contribution_list.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(&b.0)));

    println!(
        "贡献者总计 {} 人，正在生成贡献者头像画廊图",
        contribution_list.len()
    );

    // contributors.jsonl
    let mut contributor_indecies =
        std::fs::File::create(layout.metadata_dir.join("contributors.jsonl"))?;
    for (_, c) in &contribution_list {
        serde_json::to_writer(
            &mut contributor_indecies,
            &serde_json::json!({
                "githubId": c.github_id,
                "count": c.count
            }),
        )?;
        contributor_indecies.write_all(b"\n")?;
    }

    // CONTRIBUTORS.md
    let mut md_file = std::fs::File::create(layout.root.join("CONTRIBUTORS.md"))?;
    writeln!(md_file, "# 贡献者列表\n")?;
    writeln!(md_file, "> [!TIP]")?;
    writeln!(
        md_file,
        "> 本排名由机器人根据已在库歌词统计元数据信息后自动生成，贡献最多排前，同贡献量排名不分先后"
    )?;
    writeln!(md_file)?;
    writeln!(
        md_file,
        "![贡献者头像画廊](https://amll-ttml-db.stevexmh.net/contributors.png)\n"
    )?;

    for (i, (_, c)) in contribution_list.iter().enumerate() {
        writeln!(
            md_file,
            "{}. #{} (贡献次数: {})",
            i + 1,
            c.github_id,
            c.count
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let gen_folder = !std::env::args().any(|x| x == "--skip-folder");
    let push_git = !std::env::args().any(|x| x == "--skip-git");
    let t = Instant::now();

    let layout = ProjectLayout::new()?;
    layout.init_directories(gen_folder)?;

    let raw_lyrics = load_raw_lyrics(&layout.raw_dir)?;
    println!(
        "正在构建所有歌词文件夹，总计 {} 个歌词文件",
        raw_lyrics.len()
    );

    let pb = ProgressBar::new(raw_lyrics.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
            .progress_chars("##-"),
    );

    // 为了去重不同版本的歌词，需要加载所有解析后的数据进内存中，也方便并行写入文件
    // 编写此部分代码时歌词库只有 2242 份文件，内存占用约 100MB，并且在可见的未来应该不会大到无法承受
    let all_parsed_entries: Vec<Result<ParsedEntry>> = raw_lyrics
        .par_iter()
        .map(|entry| {
            let file_path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            pb.inc(1);

            let file_content = std::fs::read_to_string(&file_path)
                .with_context(|| format!("无法读取歌词文件 {file_name:?}"))?;

            let parsed_lyric = process_lyric_content(&file_content)
                .with_context(|| format!("解析歌词文件 {file_name:?} 失败"))?;

            Ok(ParsedEntry {
                path: file_path,
                file_name,
                data: parsed_lyric,
            })
        })
        .collect();

    pb.finish_with_message("解析完成");

    let mut tasks: HashMap<(Platform, String), &ParsedEntry> = HashMap::new();
    let mut contribution_map = HashMap::new();

    let raw_indecies_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(layout.metadata_dir.join("raw-lyrics-index.jsonl"))?;
    let mut raw_indecies_writer = BufWriter::new(raw_indecies_file);

    for result in &all_parsed_entries {
        match result {
            Ok(entry) => {
                serde_json::to_writer(
                    &mut raw_indecies_writer,
                    &serde_json::json!({
                        "rawLyricFile": entry.file_name,
                        "metadata": entry.data.metadata,
                    }),
                )?;
                raw_indecies_writer.write_all(b"\n")?;

                for (k, v) in &entry.data.metadata {
                    if k == "ttmlAuthorGithub" {
                        for id in v {
                            let owned_id = Cow::Owned(id.clone());
                            contribution_map
                                .entry(owned_id)
                                .and_modify(|x: &mut Contributor| x.count += 1)
                                .or_insert_with(|| Contributor {
                                    github_id: Cow::Owned(id.clone()),
                                    count: 1,
                                });
                        }
                    }

                    if gen_folder {
                        let platform = match k.as_str() {
                            "ncmMusicId" => Some(Platform::Ncm),
                            "spotifyId" => Some(Platform::Spotify),
                            "qqMusicId" => Some(Platform::Qq),
                            "appleMusicId" => Some(Platform::Am),
                            _ => None,
                        };

                        if let Some(p) = platform {
                            for id in v {
                                tasks.insert((p, id.clone()), entry);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("跳过错误文件: {e:?}");
            }
        }
    }
    raw_indecies_writer.flush()?;

    println!("正在生成 {} 个歌词文件", tasks.len());
    let write_pb = ProgressBar::new(tasks.len() as u64);
    write_pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.green/white} {pos}/{len} {msg}")?
            .progress_chars("##-"),
    );

    let task_list: Vec<_> = tasks.into_iter().collect();

    task_list.par_iter().for_each(|((platform, id), entry)| {
        write_pb.inc(1);

        let target_dir = match platform {
            Platform::Ncm => &layout.ncm_dir,
            Platform::Spotify => &layout.spotify_dir,
            Platform::Qq => &layout.qq_dir,
            Platform::Am => &layout.am_dir,
        };

        if let Err(e) = save_lyric_files_to_disk(&entry.data.lines, &entry.path, target_dir, id) {
            eprintln!("写入文件失败 {platform:?} ID {id}: {e:?}");
        }
    });

    write_pb.finish_with_message("所有文件生成完毕");

    let create_index_writer = |dir: &PathBuf| -> Result<BufWriter<std::fs::File>> {
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(dir.join("index.jsonl"))?;
        Ok(BufWriter::new(file))
    };

    let mut ncm_writer = if gen_folder {
        Some(create_index_writer(&layout.ncm_dir)?)
    } else {
        None
    };
    let mut spotify_writer = if gen_folder {
        Some(create_index_writer(&layout.spotify_dir)?)
    } else {
        None
    };
    let mut qq_writer = if gen_folder {
        Some(create_index_writer(&layout.qq_dir)?)
    } else {
        None
    };
    let mut am_writer = if gen_folder {
        Some(create_index_writer(&layout.am_dir)?)
    } else {
        None
    };

    let write_one_index = |writer: &mut Option<BufWriter<std::fs::File>>,
                           id: &str,
                           entry: &ParsedEntry|
     -> Result<()> {
        if let Some(w) = writer.as_mut() {
            serde_json::to_writer(
                w.by_ref(),
                &serde_json::json!({
                    "id": id,
                    "rawLyricFile": entry.file_name,
                    "metadata": entry.data.metadata,
                }),
            )?;
            w.write_all(b"\n")?;
        }
        Ok(())
    };

    for ((platform, id), entry) in task_list {
        match platform {
            Platform::Ncm => write_one_index(&mut ncm_writer, &id, entry)?,
            Platform::Spotify => write_one_index(&mut spotify_writer, &id, entry)?,
            Platform::Qq => write_one_index(&mut qq_writer, &id, entry)?,
            Platform::Am => write_one_index(&mut am_writer, &id, entry)?,
        }
    }

    generate_contributor_report(&layout, contribution_map)?;

    if push_git {
        if is_git_worktree_clean()? {
            println!("工作区是干净的，不需要提交。耗时: {:?}", t.elapsed());
        } else {
            println!("工作树已变更，正在提交更改");
            add_file_to_git("../..")?;
            let time = Utc::now();
            commit(&format!("于 {time:?} 重新构建更新"))?;
            push("main")?;

            println!("文件夹重建完毕！耗时: {:?}", t.elapsed());
        }
    } else {
        println!("已跳过 Git 操作，文件夹重建完毕！耗时: {:?}", t.elapsed());
    }

    Ok(())
}
