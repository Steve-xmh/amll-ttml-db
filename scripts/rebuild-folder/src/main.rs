use std::{
    borrow::Cow,
    io::{BufReader, Write},
    path::Path,
    time::Instant,
};

use amll_lyric::ttml::TTMLLyric;
use anyhow::Context;
use chrono::prelude::*;

fn is_git_worktree_clean() -> anyhow::Result<bool> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("无法执行 git status 命令")?;
    Ok(output.stdout.is_empty() && output.stderr.is_empty())
}

fn add_file_to_git(file: &str) -> anyhow::Result<()> {
    let result = std::process::Command::new("git")
        .args(["add", file])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("无法执行 git add 命令")?;
    anyhow::ensure!(result.success(), "git add 命令执行失败");
    Ok(())
}

fn commit(message: &str) -> anyhow::Result<()> {
    let result = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("无法执行 git commit 命令")?;
    anyhow::ensure!(result.success(), "git commit 命令执行失败");
    Ok(())
}

fn push(branch: &str) -> anyhow::Result<()> {
    let result = std::process::Command::new("git")
        .args(["push", "--set-upstream", "origin", branch])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("无法执行 git push 命令")?;
    anyhow::ensure!(result.success(), "git push 命令执行失败");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let gen_folder = !std::env::args().any(|x| x == "--skip-folder");
    let push_git = !std::env::args().any(|x| x == "--skip-git");

    let t = std::time::Instant::now();
    let cwd = std::env::current_dir().unwrap();
    let root_dir = cwd.join("../../");
    let raw_dir = root_dir.join("raw-lyrics");
    let ncm_dir = root_dir.join("ncm-lyrics");
    let spotify_dir = root_dir.join("spotify-lyrics");
    let qq_dir = root_dir.join("qq-lyrics");
    let am_dir = root_dir.join("am-lyrics");
    let metadata_dir = root_dir.join("metadata");
    if gen_folder {
        let _ = std::fs::remove_dir_all(&ncm_dir);
        let _ = std::fs::remove_dir_all(&spotify_dir);
        let _ = std::fs::remove_dir_all(&qq_dir);
        let _ = std::fs::remove_dir_all(&am_dir);
        std::fs::create_dir_all(&ncm_dir)?;
        std::fs::create_dir_all(&spotify_dir)?;
        std::fs::create_dir_all(&qq_dir)?;
        std::fs::create_dir_all(&am_dir)?;
    }
    let _ = std::fs::remove_dir_all(&metadata_dir);
    std::fs::create_dir_all(&metadata_dir)?;
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
    let generate_lyric_files = if gen_folder {
        |lyric: &TTMLLyric, raw_lyric_path: &Path, dest: &Path, name: &str| -> anyhow::Result<()> {
            {
                let mut indecies_file = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(dest.join("index.jsonl"))?;
                let raw_lyric_file = raw_lyric_path.file_name().map(|x| x.to_string_lossy());
                serde_json::to_writer(
                    &mut indecies_file,
                    &serde_json::json!({
                        "id": name,
                        "rawLyricFile": raw_lyric_file,
                        "metadata": lyric.metadata,
                    }),
                )?;
                indecies_file.write_all(b"\n")?;
            }

            let file_path = dest.join(name).with_extension("ttml");
            std::fs::copy(raw_lyric_path, file_path)?;
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
        }
    } else {
        |_lyric: &TTMLLyric,
         _raw_lyric_path: &Path,
         _dest: &Path,
         _name: &str|
         -> anyhow::Result<()> { Ok(()) }
    };
    let raw_lyrics_len = raw_lyrics.len();
    let raw_lyrics_char_len = raw_lyrics_len.to_string().len();
    println!("正在构建所有歌词文件夹，总计 {} 个歌词文件", raw_lyrics_len);

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Contributor<'a> {
        github_id: Cow<'a, str>,
        count: usize,
    }

    let mut contribution_map = std::collections::HashMap::new();
    let mut log_i = Instant::now();
    'lyric_parse: for (entry_i, entry) in raw_lyrics.iter().enumerate() {
        let file_path = entry.path();
        if log_i.elapsed().as_secs() >= 1 {
            log_i = Instant::now();
            println!(
                "[{:pad$}/{:pad$}] 正在解析歌词文件 {:?}",
                entry_i + 1,
                raw_lyrics.len(),
                file_path.file_name().unwrap(),
                pad = raw_lyrics_char_len
            );
        }
        let file = std::fs::File::open(&file_path)
            .with_context(|| format!("无法打开歌词文件 {:?}", entry.file_name()))?;
        let lyric_data = amll_lyric::ttml::parse_ttml(BufReader::new(file))
            .with_context(|| format!("解析歌词文件 {:?} 时出错", entry.file_name()))?;
        for line in &lyric_data.lines {
            if line.start_time > line.end_time {
                println!(
                    "[警告] 歌词文件 {:?} 中存在错误的行时间戳，跳过生成以避免恐慌发生",
                    entry.file_name()
                );
                continue 'lyric_parse;
            }
            for word in &line.words {
                if word.start_time > word.end_time {
                    println!(
                        "[警告] 歌词文件 {:?} 中存在错误的单词时间戳，跳过生成以避免恐慌发生",
                        entry.file_name()
                    );
                    continue 'lyric_parse;
                }
            }
        }
        for (key, values) in lyric_data.metadata.iter() {
            match key.as_ref() {
                "ncmMusicId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &file_path, &ncm_dir, id.as_ref())?;
                    }
                }
                "spotifyId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &file_path, &spotify_dir, id.as_ref())?;
                    }
                }
                "qqMusicId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &file_path, &qq_dir, id.as_ref())?;
                    }
                }
                "appleMusicId" => {
                    for id in values.iter() {
                        generate_lyric_files(&lyric_data, &file_path, &am_dir, id.as_ref())?;
                    }
                }
                "ttmlAuthorGithub" => {
                    for id in values.iter() {
                        contribution_map
                            .entry(Cow::clone(id))
                            .and_modify(|x: &mut Contributor| {
                                x.count += 1;
                            })
                            .or_insert_with(|| Contributor {
                                github_id: Cow::clone(id),
                                count: 1,
                            });
                    }
                }
                _ => {}
            }
        }
        // println!("文件: {}", file.file_name().to_string_lossy());
    }

    let mut contribution_list = contribution_map.into_iter().collect::<Vec<_>>();
    contribution_list.sort_by_key(|x| x.1.count);
    contribution_list.reverse();
    let contributors_count = contribution_list.len();

    println!(
        "贡献者总计 {} 人，正在生成贡献者头像画廊图",
        contributors_count
    );

    {
        let mut contributor_indecies =
            std::fs::File::create(metadata_dir.join("contributors.jsonl"))?;
        for (contributor, c) in contribution_list.iter() {
            serde_json::to_writer(
                &mut contributor_indecies,
                &serde_json::json!(
                    {
                        "githubId": contributor,
                        "count": c.count
                    }
                ),
            )?;
            contributor_indecies.write_all(b"\n")?;
        }
    }

    // 已弃用，准备使用 CloudFlare Worker 代替
    // 生成贡献者头像画廊图
    // {
    //     let avatar_size = 70;
    //     let padding = 1;
    //     let avatar_size_inner = avatar_size - padding * 2;
    //     let width = 800 / avatar_size * avatar_size;
    //     let height = (contributors_count / (width / avatar_size) + 1) * avatar_size;

    //     let mut svg_file = std::fs::File::create(root_dir.join("CONTRIBUTORS.svg"))?;

    //     writeln!(
    //         svg_file,
    //         r##"<svg version="1.1" baseProfile="full" width="{width}" height="{height}" xmlns="http://www.w3.org/2000/svg">"##
    //     )?;

    //     let mut avatar_data_buf = Vec::with_capacity(1024 * 1024 * 10);
    //     for (i, (contributor, _count)) in contribution_list.iter().enumerate() {
    //         println!("[{i}/{contributors_count}] 正在获取用户 ID {contributor} 的头像",);
    //         let x = (i % (width / avatar_size)) * avatar_size + padding;
    //         let y = (i / (width / avatar_size)) * avatar_size + padding;
    //         let avatar_url = format!(
    //             "https://avatars.githubusercontent.com/u/{}?v=4",
    //             contributor
    //         );
    //         avatar_data_buf.clear();
    //         let mut res = ureq::get(&avatar_url)
    //             .call()
    //             .context("无法下载贡献者头像")?
    //             .into_body()
    //             .into_reader();
    //         res.read_to_end(&mut avatar_data_buf)?;
    //         let avatar_data = BASE64_STANDARD.encode(&avatar_data_buf);
    //         writeln!(
    //             svg_file,
    //             r##"<image x="{x}" y="{y}" width="{avatar_size_inner}" height="{avatar_size_inner}" href="data:image;base64,{avatar_data}" style="clip-path: inset(0 0 0 0 round 50%);" />"##
    //         )?;
    //     }

    //     writeln!(svg_file, r##"</svg>"##)?;
    // }
    // 生成贡献者贡献信息
    {
        let mut md_file = std::fs::File::create(root_dir.join("CONTRIBUTORS.md"))?;

        writeln!(md_file, r##"# 贡献者列表"##)?;
        writeln!(md_file)?;
        writeln!(md_file, r##"> [!TIP]"##)?;
        writeln!(
            md_file,
            r##"> 本排名由机器人根据已在库歌词统计元数据信息后自动生成，贡献最多排前，同贡献量排名不分先后"##
        )?;
        writeln!(md_file)?;
        writeln!(md_file, r##"![贡献者头像画廊](./CONTRIBUTORS.svg)"##)?;
        writeln!(md_file)?;

        for (i, (contributor, c)) in contribution_list.iter().enumerate() {
            writeln!(
                md_file,
                r##"{}. #{contributor} (贡献次数: {})"##,
                i + 1,
                c.count
            )?;
        }
    }

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
