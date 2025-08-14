use anyhow::{Context, Result};
use octocrab::Octocrab;
use octocrab::models::IssueState;
use octocrab::models::issues::Comment;
use octocrab::models::issues::Issue;
use octocrab::params::LockReason;
use rand::distr::Alphanumeric;
use rand::distr::SampleString;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use ttml_processor::MetadataStore;
use ttml_processor::types::CanonicalMetadataKey;

use crate::git_utils;

const EXPERIMENTAL_LABEL: &str = "实验性歌词提交/修正";
const CHECKED_MARK: &str = "<!-- AMLL-DB-BOT-CHECKED -->";

pub struct PrContext<'a> {
    pub issue: &'a Issue,
    pub original_ttml: &'a str,
    pub compact_ttml: &'a str,
    pub formatted_ttml: &'a str,
    pub metadata_store: &'a MetadataStore,
    pub remarks: &'a str,
    pub warnings: &'a [String],
    pub root_path: &'a Path,
}

#[derive(Clone)]
pub struct GitHubClient {
    client: Arc<Octocrab>,
    owner: String,
    repo: String,
}

impl GitHubClient {
    pub fn new(token: String, owner: String, repo: String) -> Result<Self> {
        let client = Octocrab::builder().personal_token(token).build()?;
        Ok(Self {
            client: Arc::new(client),
            owner,
            repo,
        })
    }

    /// 检查与指定 Issue 关联的 PR 是否已存在
    ///
    /// # 参数
    /// * `issue_number` - 需要检查的 Issue 编号
    ///
    /// # 返回
    /// * `Ok(true)` - 如果已存在一个开放的、由机器人创建的 PR
    /// * `Ok(false)` - 如果不存在
    pub async fn pr_for_issue_exists(&self, issue_number: u64) -> Result<bool> {
        let head_branch = format!("auto-submit-issue-{issue_number}");
        // 构建 GitHub 搜索查询语句
        // repo:{owner}/{repo} -> 限定在当前仓库
        // is:pr -> 只搜索 PR
        // is:open -> 只搜索开启状态的 PR
        // head:{branch} -> 搜索指定 head 分支的 PR
        let query = format!(
            "repo:{}/{} is:pr is:open head:{}",
            self.owner, self.repo, head_branch
        );

        log::info!("正在搜索已存在的 PR，查询: '{query}'");

        let search_result = self
            .client
            .search()
            .issues_and_pull_requests(&query)
            .send()
            .await?;

        let count = search_result.total_count.unwrap_or(0);

        if count > 0 {
            log::info!("发现 {count} 个与 Issue #{issue_number} 关联的已存在 PR，将跳过处理。");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取所有带 "实验性歌词提交/修正" 标签的 Issue
    pub async fn list_experimental_issues(&self) -> Result<Vec<Issue>> {
        log::info!("正在请求 Issue 列表...");

        let first_page = self
            .client
            .issues(&self.owner, &self.repo)
            .list()
            .labels(&[EXPERIMENTAL_LABEL.to_string()])
            .state(octocrab::params::State::Open)
            .send()
            .await?;

        let all_issues: Vec<Issue> = self.client.all_pages(first_page).await?;

        log::info!("获取到 {} 个待处理的 Issue。", all_issues.len());
        Ok(all_issues)
    }

    /// 解析 Issue 的正文
    pub fn parse_issue_body(body: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        let mut current_key: Option<String> = None;
        let mut current_value = String::new();

        for line in body.lines() {
            if line.starts_with("### ") {
                if let Some(key) = current_key.take() {
                    params.insert(key, current_value.trim().to_string());
                }
                current_key = Some(line.trim_start_matches("### ").trim().to_string());
                current_value.clear();
            } else if current_key.is_some() {
                if let Some(stripped) = line.trim().strip_prefix("- [x] ") {
                    current_value.push_str(stripped.trim());
                    current_value.push('\n');
                } else if line.trim().starts_with("- [ ] ") {
                    // 什么也不做
                } else {
                    current_value.push_str(line.trim());
                    current_value.push('\n');
                }
            }
        }
        if let Some(key) = current_key {
            params.insert(key, current_value.trim().to_string());
        }
        params
    }

    pub async fn has_bot_commented(&self, issue_number: u64) -> Result<bool> {
        let comments_page = self
            .client
            .issues(&self.owner, &self.repo)
            .list_comments(issue_number)
            .send()
            .await?;

        let all_comments: Vec<Comment> = self.client.all_pages(comments_page).await?;

        for comment in all_comments {
            let body_matches = comment.body.as_deref().unwrap_or("").contains(CHECKED_MARK);

            if body_matches {
                let user_type_is_bot = comment.user.r#type == "Bot";
                let user_id_matches = comment.user.id.0 == 39_523_898;

                if user_type_is_bot || user_id_matches {
                    log::info!(
                        "发现来自机器人 (ID: {}, Type: {}) 的检查标记，将跳过 Issue #{}",
                        comment.user.id,
                        comment.user.r#type,
                        issue_number
                    );
                    return Ok(true);
                }
            }
        }

        // 遍历完所有评论后仍未找到匹配项
        Ok(false)
    }

    /// 发表拒绝评论并关闭 Issue
    pub async fn post_decline_comment(
        &self,
        issue_number: u64,
        reason: &str,
        ttml_content: &str,
    ) -> Result<()> {
        let body = format!(
            "{}\n\n**歌词提交议题检查失败**\n\n原因: {}\n\n```xml\n{}\n```",
            CHECKED_MARK,
            reason,
            &ttml_content[..ttml_content.len().min(65535)]
        );

        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(issue_number, &body)
            .await?;

        self.client
            .issues(&self.owner, &self.repo)
            .update(issue_number)
            .state(IssueState::Closed)
            .send()
            .await?;

        log::info!("已在 Issue #{issue_number} 发表拒绝评论并关闭。");
        Ok(())
    }

    pub async fn post_success_and_create_pr(&self, context: &PrContext<'_>) -> Result<()> {
        let issue_number = context.issue.number;

        let submit_branch = format!("auto-submit-issue-{issue_number}");
        git_utils::checkout_main_branch().await?;
        git_utils::delete_branch_if_exists(&submit_branch).await?;
        git_utils::create_branch(&submit_branch).await?;

        let unique_id = Alphanumeric.sample_string(&mut rand::rng(), 8);
        let new_filename = format!(
            "{}-{}-{}.ttml",
            chrono::Utc::now().timestamp_millis(),
            context.issue.user.id.0,
            unique_id
        );

        let raw_lyrics_dir = context.root_path.join("raw-lyrics");
        let file_path = raw_lyrics_dir.join(&new_filename);

        if !raw_lyrics_dir.exists() {
            fs::create_dir_all(&raw_lyrics_dir).await?;
        }

        fs::write(&file_path, context.compact_ttml)
            .await
            .context(format!("写入文件 {} 失败", file_path.display()))?;
        log::info!("已将处理后的歌词写入到: {}", file_path.display());

        git_utils::add_path(&file_path).await?;

        let commit_message = format!("(实验性) 提交歌曲歌词 {new_filename} #{issue_number}");
        git_utils::commit(&commit_message).await?;
        git_utils::push(&submit_branch).await?;
        git_utils::checkout_main_branch().await?;

        // --- 2. GitHub API 操作 ---

        // 构建成功评论
        let success_comment =
            Self::build_success_comment(context.original_ttml, context.formatted_ttml);
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(issue_number, success_comment)
            .await?;
        log::info!("已在 Issue #{issue_number} 发表成功评论。");

        self.client
            .issues(&self.owner, &self.repo)
            .update(issue_number)
            .state(IssueState::Closed)
            .send()
            .await?;
        self.client
            .issues(&self.owner, &self.repo)
            .lock(issue_number, Some(LockReason::Resolved))
            .await?;
        log::info!("已关闭并锁定 Issue #{issue_number}");

        let pr_body = Self::build_pr_body(context);
        let pr_title = Self::generate_pr_title(context);

        self.client
            .pulls(&self.owner, &self.repo)
            .create(&pr_title, &submit_branch, "main")
            .body(&pr_body)
            .send()
            .await?;
        log::info!("已为 Issue #{issue_number} 创建关联的 Pull Request。");

        Ok(())
    }

    // 构建成功评论的辅助函数
    fn build_success_comment(original_lyric: &str, processed_lyric: &str) -> String {
        format!(
            "{}\n\n歌词提交议题检查完毕！歌词文件没有异常！\n已自动创建歌词提交合并请求！\n请耐心等待管理员审核歌词吧！\n\n**原始歌词数据:**\n```xml\n{}\n```\n\n**转存歌词数据:**\n```xml\n{}\n```",
            CHECKED_MARK,
            &original_lyric[..original_lyric.len().min(65535)],
            &processed_lyric[..processed_lyric.len().min(65535)]
        )
    }

    /// 根据 Issue 标题和元数据生成 Pull Request 的标题。
    /// 如果 Issue 标题仅为标签或为空，则从元数据中提取信息。
    fn generate_pr_title(context: &PrContext<'_>) -> String {
        let issue_title = &context.issue.title;
        let metadata_store = context.metadata_store;

        let artists = metadata_store
            .get_multiple_values(&CanonicalMetadataKey::Artist)
            .map(|v| v.join("/"));
        let titles = metadata_store
            .get_multiple_values(&CanonicalMetadataKey::Title)
            .map(|v| v.join("/"));

        if let (Some(artist_str), Some(title_str)) = (artists, titles)
            && !artist_str.is_empty()
            && !title_str.is_empty()
        {
            let new_title = format!("[{EXPERIMENTAL_LABEL}] {artist_str} - {title_str}");
            return new_title;
        }

        issue_title.clone()
    }

    fn build_pr_body(context: &PrContext<'_>) -> String {
        const MAX_BODY_LENGTH: usize = 65536;
        const PLACEHOLDER_TEXT: &str = "```xml\n<!-- 因数据过大请自行查看变更 -->\n```";

        let issue_number = context.issue.number;
        let user_login = &context.issue.user.login;
        let metadata_store = context.metadata_store;
        let remarks = context.remarks;
        let warnings = context.warnings;
        let compact_lyric = context.compact_ttml;
        let formatted_lyric = context.formatted_ttml;

        let mut body_parts = Vec::new();

        body_parts.push(format!("### 歌词议题 (实验性流程)\n#{issue_number}"));
        body_parts.push(format!("### 歌词作者\n@{user_login}"));

        let mut add_metadata_section = |title: &str, key: &CanonicalMetadataKey| {
            if let Some(values) = metadata_store.get_multiple_values(key)
                && !values.is_empty()
            {
                body_parts.push(format!("### {title}"));
                body_parts.push(
                    values
                        .iter()
                        .map(|v| format!("- `{v}`"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
        };

        add_metadata_section("音乐名称", &CanonicalMetadataKey::Title);
        add_metadata_section("音乐作者", &CanonicalMetadataKey::Artist);
        add_metadata_section("音乐专辑名称", &CanonicalMetadataKey::Album);

        let platform_keys_and_titles = vec![
            (CanonicalMetadataKey::NcmMusicId, "歌曲关联网易云音乐 ID"),
            (CanonicalMetadataKey::QqMusicId, "歌曲关联 QQ 音乐 ID"),
            (CanonicalMetadataKey::SpotifyId, "歌曲关联 Spotify ID"),
            (
                CanonicalMetadataKey::AppleMusicId,
                "歌曲关联 Apple Music ID",
            ),
        ];

        for (key, title) in platform_keys_and_titles {
            add_metadata_section(title, &key);
        }

        if !remarks.trim().is_empty() {
            body_parts.push("### 备注".to_string());
            if remarks.trim() == "_No response_" {
                body_parts.push("*无*".to_string());
            } else {
                body_parts.push(remarks.to_string());
            }
        }

        if !warnings.is_empty() {
            let warnings_list = warnings
                .iter()
                .map(|w| format!("> - {w}"))
                .collect::<Vec<_>>()
                .join("\n");

            let warnings_section =
                format!("> [!WARNING]\n > 解析歌词文件时发现问题，详情如下:\n{warnings_list}");
            body_parts.push(warnings_section);
        }

        let base_body = body_parts.join("\n\n");
        let separator = "\n\n";

        let compact_lyric_section = format!("### 歌词文件内容\n```xml\n{compact_lyric}\n```");
        let formatted_lyric_section =
            format!("### 歌词文件内容 (已格式化)\n```xml\n{formatted_lyric}\n```");

        let placeholder_section = format!("### 歌词文件内容\n\n{PLACEHOLDER_TEXT}");
        let final_placeholder = "因数据过大，已省略歌词文本。请自行查看变更。".to_string();

        let full_body_len = base_body.len()
            + separator.len() * 2
            + compact_lyric_section.len()
            + formatted_lyric_section.len();
        if full_body_len <= MAX_BODY_LENGTH {
            return format!(
                "{base_body}{separator}{compact_lyric_section}{separator}{formatted_lyric_section}"
            );
        }

        let partial_body_len = base_body.len()
            + separator.len() * 2
            + placeholder_section.len()
            + formatted_lyric_section.len();
        if partial_body_len <= MAX_BODY_LENGTH {
            return format!(
                "{base_body}{separator}{placeholder_section}{separator}{formatted_lyric_section}"
            );
        }

        format!("{base_body}{separator}{final_placeholder}")
    }
}
