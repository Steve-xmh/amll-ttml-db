mod git_utils;
mod github_api;

use anyhow::Result;
use chrono::Utc;
use env_logger::Builder;
use log::LevelFilter;
use reqwest::Client;
use std::io::Write;
use std::path::{Path, PathBuf};
use ttml_processor::types::TtmlParsingOptions;
use ttml_processor::{
    MetadataStore, generate_ttml, parse_ttml,
    types::{TtmlGenerationOptions, TtmlTimingMode},
    validate_lyrics_and_metadata,
};

use crate::github_api::PrContext;

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
    let lyric_options = body_params.get("歌词选项").cloned().unwrap_or_default();
    let timing_mode = if lyric_options.contains("这是逐行歌词") {
        TtmlTimingMode::Line
    } else {
        TtmlTimingMode::Word
    };
    log::info!("Issue #{} 使用计时模式: {:?}", issue.number, timing_mode);

    let advanced_toggles = body_params.get("功能开关").cloned().unwrap_or_default();
    let auto_split = advanced_toggles.contains("启用自动分词");

    let punctuation_weight = if auto_split {
        log::info!("Issue #{} 已启用自动分词。", issue.number);
        body_params
            .get("[分词] 标点符号权重")
            .and_then(|s| {
                if s.is_empty() || s == "_No response_" {
                    None
                } else {
                    s.parse().ok()
                }
            })
            .unwrap_or(0.3)
    } else {
        0.3
    };

    // 3. 下载 TTML 文件
    log::info!("正在从 URL 下载 TTML: {ttml_url}");
    let original_ttml_content = http_client.get(ttml_url).send().await?.text().await?;

    log::info!("开始解析 TTML 文件...");
    let parsing_options = TtmlParsingOptions {
        force_timing_mode: Some(timing_mode),
        default_languages: ttml_processor::DefaultLanguageOptions::default(),
    };
    let mut parsed_data = match parse_ttml(&original_ttml_content, &parsing_options) {
        Ok(data) => {
            if !data.warnings.is_empty() {
                for warning in &data.warnings {
                    log::warn!("解析警告 (Issue #{}): {}", issue.number, warning);
                }
            }
            log::info!("文件解析成功。");
            data
        }
        Err(e) => {
            let err_msg = format!("解析 TTML 文件失败: `{e:?}`");
            github
                .post_decline_comment(issue.number, &err_msg, &original_ttml_content)
                .await?;
            return Ok(());
        }
    };

    parsed_data.lines.sort_by_key(|line| line.start_ms);

    let warnings = parsed_data.warnings.clone();
    if !warnings.is_empty() {
        log::warn!(
            "发现 {} 条解析警告 (Issue #{})",
            warnings.len(),
            issue.number
        );
    }

    log::info!("正在处理元数据...");
    let mut metadata_store = MetadataStore::new();

    if let Some(agent_definitions) = parsed_data.raw_metadata.get("agent")
        && !agent_definitions.is_empty()
    {
        metadata_store.set_multiple("internal::agents", agent_definitions.clone());
    }

    let other_metadata: std::collections::HashMap<_, _> = parsed_data
        .raw_metadata
        .iter()
        .filter(|(key, _)| key.as_str() != "agent")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    metadata_store.load_from_raw(&other_metadata);

    metadata_store.deduplicate_values();
    log::info!("元数据处理完毕。准备用于验证的内容: {metadata_store:?}");
    log::info!("正在验证歌词数据和元数据...");
    if let Err(errors) = validate_lyrics_and_metadata(&parsed_data.lines, &metadata_store) {
        let err_msg = format!("文件验证失败:\n- {}", errors.join("\n- "));
        github
            .post_decline_comment(issue.number, &err_msg, &original_ttml_content)
            .await?;
        return Ok(());
    }
    log::info!("文件验证通过。");

    let agent_store = ttml_processor::types::AgentStore::from_metadata_store(&metadata_store);

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
    )?;

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
    )?;

    log::info!("Issue #{} 验证通过，已生成 TTML。", issue.number);

    let pr_context = PrContext {
        issue,
        original_ttml: &original_ttml_content,
        compact_ttml: &compact_ttml,
        formatted_ttml: &formatted_ttml,
        metadata_store: &metadata_store,
        remarks: &remarks,
        warnings: &parsed_data.warnings,
        root_path,
    };

    github.post_success_and_create_pr(&pr_context).await?;
    Ok(())
}
