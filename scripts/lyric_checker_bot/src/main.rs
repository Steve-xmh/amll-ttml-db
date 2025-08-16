mod git_utils;
mod github_api;
mod validator;

use anyhow::{Context, Result};
use chrono::Utc;
use env_logger::Builder;
use log::LevelFilter;
use lyrics_helper_core::{
    DefaultLanguageOptions, MetadataStore, TtmlGenerationOptions, TtmlParsingOptions,
    TtmlTimingMode,
};
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use ttml_processor::{generate_ttml, parse_ttml};

use crate::github_api::{PrContext, PrUpdateContext};

struct TtmlProcessingOutput {
    compact_ttml: String,
    formatted_ttml: String,
    metadata_store: MetadataStore,
    warnings: Vec<String>,
}

fn process_ttml_string(
    original_ttml: &str,
    lyric_options: &str,
    advanced_toggles: &str,
    punctuation_weight_str: Option<&str>,
) -> Result<TtmlProcessingOutput, String> {
    let timing_mode = if lyric_options.contains("这是逐行歌词") {
        TtmlTimingMode::Line
    } else {
        TtmlTimingMode::Word
    };
    log::info!("使用计时模式: {:?}", timing_mode);

    let auto_split = advanced_toggles.contains("启用自动分词");

    let punctuation_weight = if auto_split {
        log::info!("已启用自动分词。");
        punctuation_weight_str
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.3)
    } else {
        0.3
    };

    log::info!("开始解析 TTML 文件...");
    let parsing_options = TtmlParsingOptions {
        force_timing_mode: Some(timing_mode),
        default_languages: DefaultLanguageOptions::default(),
    };
    let mut parsed_data = match parse_ttml(original_ttml, &parsing_options) {
        Ok(data) => {
            if !data.warnings.is_empty() {
                for warning in &data.warnings {
                    log::warn!("解析警告: {}", warning);
                }
            }
            log::info!("文件解析成功。");
            data
        }
        Err(e) => return Err(format!("解析 TTML 文件失败: `{e:?}`")),
    };

    parsed_data.lines.sort_by_key(|line| line.start_ms);

    let warnings = parsed_data.warnings.clone();
    if !warnings.is_empty() {
        log::warn!("发现 {} 条解析警告", warnings.len());
    }

    log::info!("正在处理元数据...");
    let metadata_store = MetadataStore::from(&parsed_data);

    log::info!("元数据处理完毕。准备用于验证的内容: {metadata_store:?}");
    log::info!("正在验证歌词数据和元数据...");
    if let Err(errors) =
        validator::validate_lyrics_and_metadata(&parsed_data.lines, &metadata_store)
    {
        return Err(format!("文件验证失败:\n- {}", errors.join("\n- ")));
    }
    log::info!("文件验证通过。");

    let agent_store = if parsed_data.agents.agents_by_id.is_empty() {
        MetadataStore::to_agent_store(&metadata_store)
    } else {
        parsed_data.agents.clone()
    };

    log::info!("正在生成 TTML 文件...");

    log::info!("正在生成压缩的 TTML...");
    let compact_gen_opts = TtmlGenerationOptions {
        timing_mode,
        format: false,
        auto_word_splitting: auto_split,
        punctuation_weight,
        ..Default::default()
    };
    let compact_ttml = generate_ttml(
        &parsed_data.lines,
        &metadata_store,
        &agent_store,
        &compact_gen_opts,
    )
    .map_err(|e| format!("生成压缩 TTML 失败: {e:?}"))?;

    log::info!("正在生成格式化的 TTML...");
    let formatted_gen_opts = TtmlGenerationOptions {
        timing_mode,
        format: true,
        auto_word_splitting: auto_split,
        punctuation_weight,
        ..Default::default()
    };
    let formatted_ttml = generate_ttml(
        &parsed_data.lines,
        &metadata_store,
        &agent_store,
        &formatted_gen_opts,
    )
    .map_err(|e| format!("生成格式化 TTML 失败: {e:?}"))?;

    Ok(TtmlProcessingOutput {
        compact_ttml,
        formatted_ttml,
        metadata_store,
        warnings,
    })
}

#[derive(Deserialize, Debug)]
struct CommentEventPayload {
    comment: Comment,
    issue: Issue,
}

#[derive(Deserialize, Debug)]
struct Comment {
    body: String,
    user: User,
}

#[derive(Deserialize, Debug)]
struct Issue {
    number: u64,
    #[serde(rename = "pull_request")]
    pull_request: Option<serde_json::Value>, // 仅用于判断是否存在
}

#[derive(Deserialize, Debug)]
struct User {
    login: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    Builder::from_default_env()
        .format(|buf, record| {
            let level_style = buf.default_level_style(record.level());

            writeln!(
                buf,
                "{} [{}{}{:#}] - {}",
                Utc::now().format("%Y-%m-%dT%H:%M:%S"),
                level_style,
                record.level(),
                level_style,
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    log::info!("启动实验性歌词提交检查程序...");

    let token = std::env::var("GITHUB_TOKEN").expect("未设置 GITHUB_TOKEN");
    let repo_str = std::env::var("GITHUB_REPOSITORY").expect("未设置 GITHUB_REPOSITORY");
    let (owner, repo_name) = repo_str
        .split_once('/')
        .expect("GITHUB_REPOSITORY 格式无效");
    log::info!("目标仓库: {owner}/{repo_name}");

    let workspace_root = std::env::var("GITHUB_WORKSPACE")
        .expect("错误：未设置 GITHUB_WORKSPACE 环境变量。此程序应在 GitHub Actions 环境中运行。");
    let root_path = PathBuf::from(workspace_root);

    let http_client = Client::new();
    let github = github_api::GitHubClient::new(token, owner.to_string(), repo_name.to_string())?;

    let event_name = std::env::var("GITHUB_EVENT_NAME").unwrap_or_default();
    if event_name == "issue_comment" {
        log::info!("开始处理命令...");
        if let Err(e) = handle_command(&github, &http_client, &root_path).await {
            log::error!("处理命令失败: {:?}", e);
        }
    } else {
        log::info!("开始处理新的 Issues...");
        if let Err(e) = handle_scheduled_run(github, http_client, root_path).await {
            log::error!("处理计划任务失败: {:?}", e);
        }
    }

    log::info!("任务处理完毕。");
    Ok(())
}

/// 处理由 issue_comment 事件触发的命令
async fn handle_command(
    github: &github_api::GitHubClient,
    http_client: &Client,
    root_path: &Path,
) -> Result<()> {
    let event_path =
        std::env::var("GITHUB_EVENT_PATH").context("未找到 GITHUB_EVENT_PATH，无法读取事件内容")?;
    let event_content =
        fs::read_to_string(event_path).context("无法读取 GITHUB_EVENT_PATH 指定的文件")?;

    let payload: CommentEventPayload =
        serde_json::from_str(&event_content).context("解析评论事件 JSON 失败")?;

    if payload.issue.pull_request.is_none() {
        log::info!("评论不在 Pull Request 中，已忽略。");
        return Ok(());
    }

    let pr_number = payload.issue.number;
    let commenter = &payload.comment.user.login;
    let body = payload.comment.body.trim();

    log::info!(
        "在 PR #{} 中收到来自 @{} 的评论: '{}'",
        pr_number,
        commenter,
        body
    );

    if let Some(reason) = body.strip_prefix("/close") {
        github
            .close_pr_for_user(
                pr_number,
                commenter,
                Some(reason.trim()).filter(|s| !s.is_empty()),
            )
            .await
    } else if let Some(url) = body
        .strip_prefix("/update")
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let original_ttml_content = match http_client.get(url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    let err_msg = format!("@{commenter}，无法读取你的 TTML: {e:?}");
                    github.post_comment(pr_number, &err_msg).await?;
                    return Ok(());
                }
            },
            Err(e) => {
                let err_msg = format!("@{commenter}，下载你的 TTML 文件失败: {e:?}");
                github.post_comment(pr_number, &err_msg).await?;
                return Ok(());
            }
        };

        let (lyric_options, advanced_toggles, punctuation_weight_str) =
            match github.get_options_from_original_issue(pr_number).await? {
                Some(opts) => (
                    opts.lyric_options,
                    opts.advanced_toggles,
                    opts.punctuation_weight_str,
                ),
                None => {
                    let err_msg =
                        format!("@{commenter}，无法从此 PR 找到关联的原始 Issue 来获取解析选项。");
                    github.post_comment(pr_number, &err_msg).await?;
                    return Ok(());
                }
            };

        match process_ttml_string(
            &original_ttml_content,
            &lyric_options,
            &advanced_toggles,
            punctuation_weight_str.as_deref(),
        ) {
            Ok(processed_data) => {
                let update_context = PrUpdateContext {
                    pr_number,
                    original_ttml: &original_ttml_content,
                    compact_ttml: &processed_data.compact_ttml,
                    formatted_ttml: &processed_data.formatted_ttml,
                    warnings: &processed_data.warnings,
                    root_path,
                    requester: commenter,
                };
                github.update_pr(&update_context).await?;
            }
            Err(err_msg) => {
                github
                    .post_pr_failure_comment(pr_number, commenter, &err_msg, &original_ttml_content)
                    .await?;
            }
        }
        Ok(())
    } else {
        log::info!("评论不包含已知命令，已忽略。");
        Ok(())
    }
}

/// 按计划执行，检查所有待处理的 Issues
async fn handle_scheduled_run(
    github: github_api::GitHubClient,
    http_client: Client,
    root_path: PathBuf,
) -> Result<()> {
    log::info!("正在获取带 '实验性歌词提交/修正' 标签的 Issue...");
    let issues = github.list_experimental_issues().await?;

    for issue in issues {
        let http_client = http_client.clone();
        let github = github.clone();
        let root_path = root_path.clone();

        log::info!("开始处理 Issue #{}: {}", issue.number, issue.title);
        if let Err(e) = process_issue(&issue, http_client, github, &root_path).await {
            log::error!("处理 Issue #{} 失败: {:?}", issue.number, e);
        }
    }

    log::info!("所有 Issue 处理完毕。");
    Ok(())
}

/// 处理单个 Issue
async fn process_issue(
    issue: &octocrab::models::issues::Issue,
    http_client: Client,
    github: github_api::GitHubClient,
    root_path: &Path,
) -> Result<()> {
    if github.pr_for_issue_exists(issue.number).await? {
        // 如果 PR 已存在，直接返回，不再处理
        return Ok(());
    }

    // 检查是否已处理
    if github.has_bot_commented(issue.number).await? {
        log::info!("Issue #{} 已被机器人评论过，跳过。", issue.number);
        return Ok(());
    }

    // 2. 解析 Issue Body
    let issue_body = issue.body.as_deref().unwrap_or("");
    let body_params = crate::github_api::GitHubClient::parse_issue_body(issue_body);
    let ttml_url = match body_params.get("TTML 歌词文件下载直链") {
        Some(url) if !url.is_empty() => url,
        _ => {
            github
                .post_decline_comment(
                    issue.number,
                    "无法在 Issue 中找到有效的“TTML 歌词文件下载直链”。",
                    "",
                )
                .await?;
            return Ok(());
        }
    };
    let remarks = body_params.get("备注").cloned().unwrap_or_default();

    // 解析歌词选项
    let options = crate::github_api::GitHubClient::extract_options_from_body(&body_params);

    // 3. 下载 TTML 文件
    log::info!("正在从 URL 下载 TTML: {ttml_url}");
    let original_ttml_content = match http_client.get(ttml_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(text) => text,
            Err(e) => {
                let err_msg = format!("无法读取 TTML 响应内容: {e:?}");
                github
                    .post_decline_comment(issue.number, &err_msg, "")
                    .await?;
                return Ok(());
            }
        },
        Err(e) => {
            let err_msg = format!("下载 TTML 文件失败: {e:?}");
            github
                .post_decline_comment(issue.number, &err_msg, "")
                .await?;
            return Ok(());
        }
    };

    match process_ttml_string(
        &original_ttml_content,
        &options.lyric_options,
        &options.advanced_toggles,
        options.punctuation_weight_str.as_deref(),
    ) {
        Ok(processed_data) => {
            log::info!("Issue #{} 验证通过，已生成 TTML。", issue.number);

            let pr_context = PrContext {
                issue,
                original_ttml: &original_ttml_content,
                compact_ttml: &processed_data.compact_ttml,
                formatted_ttml: &processed_data.formatted_ttml,
                metadata_store: &processed_data.metadata_store,
                remarks: &remarks,
                warnings: &processed_data.warnings,
                root_path,
            };

            github.post_success_and_create_pr(&pr_context).await?;
        }
        Err(err_msg) => {
            github
                .post_decline_comment(issue.number, &err_msg, &original_ttml_content)
                .await?;
        }
    }
    Ok(())
}
