use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{Context, Result};
use lyrics_helper_core::{CanonicalMetadataKey, MetadataStore};
use octocrab::{
    Octocrab,
    models::{
        IssueState,
        issues::{Comment, Issue},
        reactions::ReactionContent,
    },
    params::{LockReason, repos::Reference},
};
use rand::distr::{Alphanumeric, SampleString};
use tokio::fs;
use tracing::{error, info, warn};

use crate::git_utils;

const SUBMISSION_LABEL: &str = "歌词提交/补正";
const CHECKED_MARK: &str = "<!-- AMLL-DB-BOT-CHECKED -->";
const FIRST_TIME_LABEL: &str = "首次投稿";

pub struct PrContext<'a> {
    pub issue: &'a Issue,
    pub original_ttml: &'a str,
    pub compact_ttml: &'a str,
    pub metadata_store: &'a MetadataStore,
    pub remarks: &'a str,
    pub warnings: &'a [String],
    pub root_path: &'a Path,
    pub is_first_time: bool,
}

pub struct PrUpdateContext<'a> {
    pub pr_number: u64,
    pub compact_ttml: &'a str,
    pub warnings: &'a [String],
    pub root_path: &'a Path,
    pub requester: &'a str,
}

#[derive(Clone)]
pub struct GitHubClient {
    pub client: Arc<Octocrab>,
    pub owner: String,
    pub repo: String,
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

        info!("正在搜索已存在的 PR，查询: '{query}'");

        let search_result = self
            .client
            .search()
            .issues_and_pull_requests(&query)
            .send()
            .await?;

        let count = search_result.total_count.unwrap_or(0);

        if count > 0 {
            info!("发现 {count} 个与 Issue #{issue_number} 关联的已存在 PR，将跳过处理。");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取所有带 "实验性歌词提交/修正" 标签的 Issue
    pub async fn list_experimental_issues(&self) -> Result<Vec<Issue>> {
        info!("正在请求 Issue 列表...");

        let first_page = self
            .client
            .issues(&self.owner, &self.repo)
            .list()
            .labels(&[SUBMISSION_LABEL.to_string()])
            .state(octocrab::params::State::Open)
            .send()
            .await?;

        let all_issues: Vec<Issue> = self.client.all_pages(first_page).await?;

        info!("获取到 {} 个待处理的 Issue。", all_issues.len());
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
                    info!(
                        "发现来自机器人 (ID: {}, Type: {}) 的检查标记，将跳过 Issue #{}",
                        comment.user.id, comment.user.r#type, issue_number
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
        let base_text = format!("{CHECKED_MARK}\n\n**歌词提交议题检查失败**\n\n原因: {reason}");

        let body = Self::build_body(&base_text, Some(ttml_content), 65535);

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

        info!("已在 Issue #{issue_number} 发表拒绝评论并关闭。");
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
        info!("已将处理后的歌词写入到: {}", file_path.display());

        git_utils::add_path(&file_path).await?;

        let commit_message = format!("提交歌曲歌词 {new_filename} #{issue_number}");
        git_utils::commit(&commit_message).await?;
        git_utils::push(&submit_branch).await?;
        git_utils::checkout_main_branch().await?;

        // --- 2. GitHub API 操作 ---

        // 构建成功评论
        let success_comment =
            Self::build_issue_success_comment(context.original_ttml, context.warnings);

        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(issue_number, success_comment)
            .await?;
        info!("已在 Issue #{issue_number} 发表成功评论。");

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
        info!("已关闭并锁定 Issue #{issue_number}");

        let pr_body = Self::build_pr_body(context);
        let pr_title = Self::generate_pr_title(context);

        let pr = self
            .client
            .pulls(&self.owner, &self.repo)
            .create(&pr_title, &submit_branch, "main")
            .body(&pr_body)
            .send()
            .await?;

        info!("已为 Issue #{issue_number} 创建关联的 Pull Request。");

        if context.is_first_time {
            info!(
                "用户 {} 为首次投稿，正在添加标签...",
                context.issue.user.login
            );
            if let Err(e) = self
                .client
                .issues(&self.owner, &self.repo)
                .add_labels(pr.number, &[FIRST_TIME_LABEL.to_string()])
                .await
            {
                error!("添加首次投稿标签失败: {e:?}");
            }
        }

        Ok(())
    }

    /// 根据 Issue 标题和元数据生成 Pull Request 的标题。
    /// 如果 Issue 标题仅为标签或为空，则从元数据中提取信息。
    fn generate_pr_title(context: &PrContext<'_>) -> String {
        let issue_title = &context.issue.title;
        let placeholder_title = format!("[{SUBMISSION_LABEL}]");

        let trimmed_title = issue_title.trim();
        if trimmed_title.is_empty() || trimmed_title == placeholder_title {
            let metadata_store = context.metadata_store;
            let artists = metadata_store
                .get_multiple_values(&CanonicalMetadataKey::Artist)
                .map(|v| v.join(", "));
            let titles = metadata_store
                .get_multiple_values(&CanonicalMetadataKey::Title)
                .map(|v| v.join(", "));

            if let (Some(artist_str), Some(title_str)) = (artists, titles)
                && !artist_str.is_empty()
                && !title_str.is_empty()
            {
                return format!("[{SUBMISSION_LABEL}] {artist_str} - {title_str}");
            }
        }

        issue_title.clone()
    }

    fn build_pr_body(context: &PrContext<'_>) -> String {
        let issue_number = context.issue.number;
        let user_login = &context.issue.user.login;
        let metadata_store = context.metadata_store;
        let remarks = context.remarks;
        let warnings = context.warnings;

        let mut body_parts = Vec::new();

        body_parts.push(format!("### 歌词议题\n#{issue_number}"));
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

        let trimmed_remarks = remarks.trim();
        if !trimmed_remarks.is_empty() && trimmed_remarks != "_No response_" {
            body_parts.push("### 备注".to_string());
            body_parts.push(remarks.to_string());
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

        body_parts.join("\n\n")
    }

    pub async fn add_labels_to_pr(
        &self,
        pr_number: u64,
        requester: &str,
        labels_str: &str,
        comment_id: u64,
    ) -> Result<()> {
        if self
            .verify_pr_permission(pr_number, requester)
            .await?
            .is_none()
        {
            return Ok(());
        }

        let labels: Vec<String> = labels_str.split_whitespace().map(String::from).collect();

        if labels.is_empty() {
            return Ok(());
        }

        self.client
            .issues(&self.owner, &self.repo)
            .add_labels(pr_number, &labels)
            .await?;

        self.client
            .issues(&self.owner, &self.repo)
            .create_comment_reaction(comment_id, ReactionContent::PlusOne)
            .await?;

        Ok(())
    }

    pub async fn close_pr_for_user(
        &self,
        pr_number: u64,
        requester: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        info!("正在处理来自 @{requester} 的关闭 PR #{pr_number} 请求...");

        let Some(pr) = self.verify_pr_permission(pr_number, requester).await? else {
            return Ok(());
        };

        let branch_name = pr.head.ref_field;

        let reason_text = reason.unwrap_or("无");
        let comment_body =
            format!("应用户 @{requester} 的请求，此 PR 已关闭。\n\n**原因**: {reason_text}");
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(pr_number, comment_body)
            .await?;
        info!("已在 PR #{pr_number} 发表关闭评论。");

        self.client
            .issues(&self.owner, &self.repo)
            .update(pr_number)
            .state(IssueState::Closed)
            .send()
            .await?;
        info!("已关闭 PR #{}", pr_number);

        let branch_ref = Reference::Branch(branch_name.clone());

        match (*self.client)
            .repos(&self.owner, &self.repo)
            .delete_ref(&branch_ref)
            .await
        {
            Ok(()) => info!("成功删除分支: {branch_name}"),
            Err(e) => warn!("删除分支 {branch_name} 失败: {e:?}"),
        }

        Ok(())
    }

    /// 从 PR 的正文中解析出关联的 Issue 编号
    fn parse_issue_number_from_pr_body(body: Option<&str>) -> Option<u64> {
        let body = body?;
        for line in body.lines() {
            if let Some(stripped) = line.trim().strip_prefix('#')
                && let Ok(number) = stripped.parse::<u64>()
            {
                return Some(number);
            }
        }
        None
    }

    pub async fn update_pr(&self, context: &PrUpdateContext<'_>) -> Result<()> {
        info!(
            "正在处理来自 @{} 的更新 PR #{} 请求...",
            context.requester, context.pr_number
        );

        // 权限检查
        let Some(pr) = self
            .verify_pr_permission(context.pr_number, context.requester)
            .await?
        else {
            return Ok(());
        };

        // 找到要更新的文件
        let files = self
            .client
            .pulls(&self.owner, &self.repo)
            .list_files(context.pr_number)
            .await?
            .items;
        let ttml_file = files.iter().find(|f| {
            std::path::Path::new(&f.filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ttml"))
                && f.filename.starts_with("raw-lyrics/")
        });

        let file_to_update = if let Some(file) = ttml_file {
            context.root_path.join(&file.filename)
        } else {
            error!("在 PR #{} 中未找到 .ttml 文件", context.pr_number);
            let error_comment = format!(
                "抱歉，@{requester}，无法在此 PR 中找到需要更新的 TTML 文件。",
                requester = context.requester
            );
            self.client
                .issues(&self.owner, &self.repo)
                .create_comment(context.pr_number, error_comment)
                .await?;
            return Ok(());
        };
        info!(
            "将在 PR #{} 中更新文件: {}",
            context.pr_number,
            file_to_update.display()
        );

        // Git 操作
        let branch_name = &pr.head.ref_field;
        git_utils::checkout_main_branch().await?;
        git_utils::checkout_branch(branch_name).await?;
        git_utils::pull_branch(branch_name)
            .await
            .context("拉取分支失败")?;

        fs::write(&file_to_update, context.compact_ttml)
            .await
            .context(format!("写入文件 {} 失败", file_to_update.display()))?;
        info!("已将更新后的歌词写入到: {}", file_to_update.display());

        git_utils::add_path(&file_to_update).await?;

        if !git_utils::has_staged_changes().await? {
            let no_change_comment = format!(
                "@{requester}，你提供的新歌词文件与当前版本完全相同，无需更新。",
                requester = context.requester
            );
            self.post_comment(context.pr_number, &no_change_comment)
                .await?;
            git_utils::checkout_main_branch().await?;
            return Ok(());
        }

        let commit_message = format!("更新歌词文件内容\n\n由 @{} 请求更新。", context.requester);
        git_utils::commit(&commit_message).await?;
        git_utils::force_push(branch_name).await?;
        git_utils::checkout_main_branch().await?;

        // 发表评论
        let mut base_text = format!(
            "@{requester}，歌词文件已根据你的请求更新！",
            requester = context.requester
        );
        if !context.warnings.is_empty() {
            let warnings_list = context
                .warnings
                .iter()
                .map(|w| format!("> - {w}"))
                .collect::<Vec<_>>()
                .join("\n");
            let warnings_section =
                format!("\n\n> [!WARNING]\n> 解析歌词文件时发现以下问题:\n{warnings_list}");
            base_text.push_str(&warnings_section);
        }

        let update_comment = Self::build_body(&base_text, None, 65535);

        self.post_comment(context.pr_number, &update_comment)
            .await?;

        info!("成功更新 PR #{} 并发表了评论。", context.pr_number);

        Ok(())
    }

    /// 发表评论
    pub async fn post_comment(&self, issue_or_pr_number: u64, body: &str) -> Result<()> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(issue_or_pr_number, body)
            .await?;
        Ok(())
    }

    /// 验证用户是否是原始 Issue 作者
    async fn verify_pr_permission(
        &self,
        pr_number: u64,
        requester: &str,
    ) -> Result<Option<octocrab::models::pulls::PullRequest>> {
        let is_collaborator = self
            .client
            .repos(&self.owner, &self.repo)
            .is_collaborator(requester)
            .await
            .unwrap_or(false);

        let pr = self
            .client
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await?;

        if is_collaborator {
            return Ok(Some(pr));
        }

        if let Some(issue_number) = Self::parse_issue_number_from_pr_body(pr.body.as_deref()) {
            let original_issue = self
                .client
                .issues(&self.owner, &self.repo)
                .get(issue_number)
                .await?;
            let original_author = &original_issue.user.login;

            if requester != original_author {
                warn!(
                    "@{requester} 尝试操作 PR #{pr_number}，但 PR 的原始作者是 @{original_author}"
                );
                let error_comment = format!(
                    "@{requester}，你没有权限执行此操作。\n只有这个歌词提交的原始作者 (@{original_author}) 或仓库协作者才能操作此 PR。"
                );
                self.post_comment(pr_number, &error_comment).await?;
                return Ok(None);
            }
        } else {
            error!("无法从 PR #{} 的正文中解析出原始 Issue 编号。", pr_number);
            let error_comment = format!(
                "@{requester}，操作失败。\n无法从此 PR 追溯到原始的歌词提交议题，因此无法验证你的权限。"
            );
            self.post_comment(pr_number, &error_comment).await?;
            return Ok(None);
        }

        Ok(Some(pr))
    }

    pub async fn post_pr_failure_comment(
        &self,
        pr_number: u64,
        requester: &str,
        reason: &str,
        ttml_content: &str,
    ) -> Result<()> {
        let base_text = format!("@{requester}，你提交的歌词文件更新失败。\n\n**原因**: {reason}");

        let failure_comment = Self::build_body(&base_text, Some(ttml_content), 65535);

        self.post_comment(pr_number, &failure_comment).await
    }

    fn build_body(base_text: &str, original_lyric: Option<&str>, max_len: usize) -> String {
        const PLACEHOLDER_TEXT: &str = "```xml\n<!-- 因数据过大请自行查看变更 -->\n```";
        let separator = "\n\n";

        let original_section_title = "**原始歌词数据:**";

        // 尝试包含所有内容
        let body = base_text.to_string();
        let original_section = original_lyric
            .map(|s| format!("{separator}{original_section_title}{separator}\n```xml\n{s}\n```"));

        let mut final_body = body.clone();
        if let Some(ref section) = original_section {
            final_body.push_str(section);
        }

        if final_body.len() <= max_len {
            return final_body;
        }

        if original_lyric.is_some() {
            let placeholder_original =
                format!("{separator}{original_section_title}{separator}{PLACEHOLDER_TEXT}");

            final_body.push_str(&placeholder_original);
        }

        if final_body.len() <= max_len {
            final_body
        } else {
            body
        }
    }

    // 构建在 Issue 中发表的成功评论
    fn build_issue_success_comment(original_lyric: &str, warnings: &[String]) -> String {
        let mut base_text = format!(
            "{CHECKED_MARK}\n\n歌词提交议题检查完毕！\n已自动创建歌词提交合并请求！\n请耐心等待管理员审核歌词吧！"
        );

        if !warnings.is_empty() {
            let warnings_list = warnings
                .iter()
                .map(|w| format!("> - {w}"))
                .collect::<Vec<_>>()
                .join("\n");
            let warnings_section =
                format!("\n\n> [!WARNING]\n> 解析歌词文件时发现以下问题:\n{warnings_list}");
            base_text.push_str(&warnings_section);
        }

        Self::build_body(&base_text, Some(original_lyric), 65535)
    }
}
