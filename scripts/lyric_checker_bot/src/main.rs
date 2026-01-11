mod git_utils;
mod github_api;
mod validator;

use std::fs;
use std::io::BufRead;
use std::io::BufReader;

use anyhow::{Context, Result};
use lyrics_helper_core::{
    DefaultLanguageOptions, MetadataStore, TtmlGenerationOptions, TtmlParsingOptions,
};
use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use ttml_processor::{generate_ttml, parse_ttml};

use crate::github_api::{PrContext, PrUpdateContext};

struct TtmlProcessingOutput {
    compact_ttml: String,
    metadata_store: MetadataStore,
    warnings: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ContributorEntry {
    count: u64,

    #[serde(rename = "githubId")]
    github_id: String,

    #[serde(rename = "githubLogin")]
    github_username: String,
}

/// 从贡献者名单中寻找给定 GitHub ID 的用户，返回 False 代表不在名单中
///
/// 一个典型的应用是判断用户是否是第一次提交以添加 “首次投稿” 标签
fn check_is_contributor(root_path: &Path, user_id: u64) -> bool {
    let contributors_path = root_path.join("metadata/contributors.jsonl");

    if !contributors_path.exists() {
        error!("未能找到贡献者名单文件");
        return true;
    }

    let file = match fs::File::open(&contributors_path) {
        Ok(f) => f,
        Err(e) => {
            warn!("无法打开贡献者文件: {e:?}");
            return true;
        }
    };

    let reader = BufReader::new(file);
    let target_id_str = user_id.to_string();

    for line in reader.lines() {
        match line {
            Ok(content) => {
                if let Ok(entry) = serde_json::from_str::<ContributorEntry>(&content)
                    && entry.github_id == target_id_str
                {
                    return true;
                }
            }
            Err(e) => warn!("读取贡献者行失败: {e:?}"),
        }
    }

    false
}

fn process_ttml_string(original_ttml: &str) -> Result<TtmlProcessingOutput, String> {
    info!("开始解析 TTML 文件...");
    let parsing_options = TtmlParsingOptions {
        force_timing_mode: None,
        default_languages: DefaultLanguageOptions::default(),
    };
    let mut parsed_data = match parse_ttml(original_ttml, &parsing_options) {
        Ok(data) => {
            if !data.warnings.is_empty() {
                for warning in &data.warnings {
                    warn!("解析警告: {warning}");
                }
            }
            info!("文件解析成功。");
            data
        }
        Err(e) => return Err(format!("解析 TTML 文件失败: `{e:?}`")),
    };

    parsed_data.lines.sort_by_key(|line| line.start_ms);

    let warnings = parsed_data.warnings.clone();
    if !warnings.is_empty() {
        warn!("发现 {} 条解析警告", warnings.len());
    }

    info!("正在处理元数据...");
    let metadata_store = MetadataStore::from(&parsed_data);

    info!("元数据处理完毕。准备用于验证的内容: {metadata_store:?}");
    info!("正在验证歌词数据和元数据...");
    if let Err(errors) =
        validator::validate_lyrics_and_metadata(&parsed_data.lines, &metadata_store)
    {
        return Err(format!("文件验证失败:\n- {}", errors.join("\n- ")));
    }
    info!("文件验证通过。");

    let agent_store = &parsed_data.agents;

    info!("正在生成 TTML 文件...");
    let compact_gen_opts = TtmlGenerationOptions {
        format: false,
        ..Default::default()
    };
    let compact_ttml = generate_ttml(
        &parsed_data.lines,
        &metadata_store,
        agent_store,
        &compact_gen_opts,
    )
    .map_err(|e| format!("生成 TTML 失败: {e:?}"))?;

    Ok(TtmlProcessingOutput {
        compact_ttml,
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
    id: u64,
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

#[derive(Deserialize, Debug)]
struct IssueEventPayload {
    issue: Issue,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("lyric_checker_bot=trace"));
    let _ = FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();

    let token = std::env::var("GITHUB_TOKEN").expect("未设置 GITHUB_TOKEN");
    let repo_str = std::env::var("GITHUB_REPOSITORY").expect("未设置 GITHUB_REPOSITORY");
    let (owner, repo_name) = repo_str
        .split_once('/')
        .expect("GITHUB_REPOSITORY 格式无效");

    let workspace_root = std::env::var("GITHUB_WORKSPACE")
        .expect("错误：未设置 GITHUB_WORKSPACE 环境变量。此程序应在 GitHub Actions 环境中运行。");
    let root_path = PathBuf::from(workspace_root);

    let http_client = Client::new();
    let github = github_api::GitHubClient::new(token, owner.to_string(), repo_name.to_string())?;

    let event_name = std::env::var("GITHUB_EVENT_NAME").unwrap_or_default();

    match event_name.as_str() {
        "issue_comment" => {
            info!("处理 Issue 评论");
            if let Err(e) = Box::pin(handle_command(&github, &http_client, &root_path)).await {
                error!("处理 Issue 评论失败: {e:?}");
            }
        }
        "issues" => {
            info!("处理单个 Issue");
            if let Err(e) = handle_single_issue_event(&github, &http_client, &root_path).await {
                error!("处理单个 Issue 失败: {e:?}");
            }
        }
        _ => {
            info!("扫描全部 issue (Event: {event_name})",);
            if let Err(e) = Box::pin(handle_scheduled_run(github, http_client, root_path)).await {
                error!("扫描全部 issue 失败: {e:?}");
            }
        }
    }

    info!("任务处理完毕。");
    Ok(())
}

/// 处理由 `issue_comment` 事件触发的命令
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
        info!("评论不在 Pull Request 中，已忽略。");
        return Ok(());
    }

    let pr_number = payload.issue.number;
    let commenter = &payload.comment.user.login;
    let body = payload.comment.body.trim();
    let comment_id = payload.comment.id;

    info!(
        "在 PR #{} 中收到来自 @{} 的评论: '{}'",
        pr_number, commenter, body
    );

    if let Some(reason) = body.strip_prefix("/close") {
        github
            .close_pr_for_user(
                pr_number,
                commenter,
                Some(reason.trim()).filter(|s| !s.is_empty()),
            )
            .await
    } else if let Some(labels_str) = body.strip_prefix("/label") {
        github
            .add_labels_to_pr(pr_number, commenter, labels_str.trim(), comment_id)
            .await
    } else if let Some(args) = body.strip_prefix("/update") {
        let args = args.trim();

        if args.is_empty() {
            return Ok(());
        }

        let (url, new_remarks) = match args.split_once(char::is_whitespace) {
            Some((u, r)) => (u, Some(r.trim().to_string())), // 有 URL 也有备注
            None => (args, None),                            // 只有 URL，没有备注
        };

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

        match process_ttml_string(&original_ttml_content) {
            Ok(processed_data) => {
                let update_context = PrUpdateContext {
                    pr_number,
                    compact_ttml: &processed_data.compact_ttml,
                    warnings: &processed_data.warnings,
                    root_path,
                    requester: commenter,
                    metadata_store: &processed_data.metadata_store,
                    remarks: new_remarks,
                    comment_id,
                };
                Box::pin(github.update_pr(&update_context)).await?;
            }
            Err(err_msg) => {
                github
                    .post_pr_failure_comment(pr_number, commenter, &err_msg, &original_ttml_content)
                    .await?;
            }
        }
        Ok(())
    } else {
        info!("评论不包含已知命令，已忽略。");
        Ok(())
    }
}

async fn handle_single_issue_event(
    github: &github_api::GitHubClient,
    http_client: &Client,
    root_path: &Path,
) -> Result<()> {
    let event_path = std::env::var("GITHUB_EVENT_PATH").context("未找到 GITHUB_EVENT_PATH")?;
    let event_content = fs::read_to_string(event_path).context("无法读取事件文件")?;

    let payload: IssueEventPayload = serde_json::from_str(&event_content)?;

    let issue = payload.issue;

    let full_issue = github
        .client
        .issues(&github.owner, &github.repo)
        .get(issue.number)
        .await
        .context("无法从 GitHub API 获取 Issue 详情")?;

    process_issue(&full_issue, http_client.clone(), github.clone(), root_path).await
}

/// 按计划执行，检查所有待处理的 Issues
async fn handle_scheduled_run(
    github: github_api::GitHubClient,
    http_client: Client,
    root_path: PathBuf,
) -> Result<()> {
    let issues = github.list_experimental_issues().await?;

    for issue in issues {
        let http_client = http_client.clone();
        let github = github.clone();
        let root_path = root_path.clone();

        info!("开始处理 Issue #{}: {}", issue.number, issue.title);
        if let Err(e) = process_issue(&issue, http_client, github, &root_path).await {
            error!("处理 Issue #{} 失败: {:?}", issue.number, e);
        }
    }

    info!("所有 Issue 处理完毕。");
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
        info!("Issue #{} 已被机器人评论过，跳过。", issue.number);
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

    // 3. 下载 TTML 文件
    info!("正在从 URL 下载 TTML: {ttml_url}");
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

    match process_ttml_string(&original_ttml_content) {
        Ok(processed_data) => {
            info!("Issue #{} 验证通过，已生成 TTML。", issue.number);

            let is_contributor = check_is_contributor(root_path, issue.user.id.0);
            let is_first_time = !is_contributor;

            let pr_context = PrContext {
                issue,
                original_ttml: &original_ttml_content,
                compact_ttml: &processed_data.compact_ttml,
                metadata_store: &processed_data.metadata_store,
                remarks: &remarks,
                warnings: &processed_data.warnings,
                root_path,
                is_first_time,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn setup_test_env(dir_name: &str) -> PathBuf {
        let mut temp_dir = env::temp_dir();
        temp_dir.push(dir_name);

        if temp_dir.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        let _ = fs::create_dir_all(temp_dir.join("metadata"));

        temp_dir
    }

    fn cleanup_test_env(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn test_check_is_contributor_detection() {
        let root_path = setup_test_env("test_lyric_bot_contributors");
        let file_path = root_path.join("metadata/contributors.jsonl");

        let content = r#"
{"count":338,"githubId":"1001","githubLogin":"OldUserA"}
{"count":251,"githubId":"1002","githubLogin":"OldUserB"}
"#;
        fs::write(&file_path, content).expect("无法写入测试文件");

        assert!(
            check_is_contributor(&root_path, 1001),
            "老贡献者 (1001) 应该被识别出来"
        );

        assert!(
            !check_is_contributor(&root_path, 9999),
            "新用户 (9999) 不应该被识别为贡献者"
        );

        cleanup_test_env(&root_path);
    }

    #[test]
    fn test_check_is_contributor_missing_file() {
        let root_path = setup_test_env("test_lyric_bot_missing_file");

        assert!(
            check_is_contributor(&root_path, 12345),
            "文件丢失时，应默认为老贡献者以避免误打标签"
        );

        cleanup_test_env(&root_path);
    }
}
