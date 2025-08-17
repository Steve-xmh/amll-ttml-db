use anyhow::{Context, Result};
use lyrics_helper_core::CanonicalMetadataKey;
use lyrics_helper_core::MetadataStore;
use octocrab::Octocrab;
use octocrab::models::IssueState;
use octocrab::models::issues::Comment;
use octocrab::models::issues::Issue;
use octocrab::params::LockReason;
use octocrab::params::repos::Reference;
use rand::distr::Alphanumeric;
use rand::distr::SampleString;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

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

pub struct PrUpdateContext<'a> {
    pub pr_number: u64,
    pub original_ttml: &'a str,
    pub compact_ttml: &'a str,
    pub formatted_ttml: &'a str,
    pub warnings: &'a [String],
    pub root_path: &'a Path,
    pub requester: &'a str,
}

pub struct OriginalIssueOptions {
    pub lyric_options: String,
    pub advanced_toggles: String,
    pub punctuation_weight_str: Option<String>,
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
        let base_text = format!(
            "{}\n\n**歌词提交议题检查失败**\n\n原因: {}",
            CHECKED_MARK, reason
        );

        let body = Self::build_body(&base_text, Some(ttml_content), None, 65535);

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
        let success_comment = Self::build_issue_success_comment(
            context.original_ttml,
            context.formatted_ttml,
            context.warnings,
        );

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

    /// 根据 Issue 标题和元数据生成 Pull Request 的标题。
    /// 如果 Issue 标题仅为标签或为空，则从元数据中提取信息。
    fn generate_pr_title(context: &PrContext<'_>) -> String {
        let issue_title = &context.issue.title;
        let placeholder_title = format!("[{EXPERIMENTAL_LABEL}]");

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
                return format!("[{EXPERIMENTAL_LABEL}] {artist_str} - {title_str}");
            }
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

    pub async fn close_pr_for_user(
        &self,
        pr_number: u64,
        requester: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        log::info!(
            "正在处理来自 @{} 的关闭 PR #{} 请求...",
            requester,
            pr_number
        );

        let pr = match self.verify_pr_permission(pr_number, requester).await? {
            Some(pr) => pr,
            None => return Ok(()),
        };

        let branch_name = pr.head.ref_field;

        let reason_text = reason.unwrap_or("无");
        let comment_body = format!(
            "应用户 @{} 的请求，此 PR 已关闭。\n\n**原因**: {}",
            requester, reason_text
        );
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(pr_number, comment_body)
            .await?;
        log::info!("已在 PR #{} 发表关闭评论。", pr_number);

        self.client
            .issues(&self.owner, &self.repo)
            .update(pr_number)
            .state(IssueState::Closed)
            .send()
            .await?;
        log::info!("已关闭 PR #{}", pr_number);

        let branch_ref = Reference::Branch(branch_name.to_string());

        match (*self.client)
            .repos(&self.owner, &self.repo)
            .delete_ref(&branch_ref)
            .await
        {
            Ok(_) => log::info!("成功删除分支: {}", branch_name),
            Err(e) => log::warn!("删除分支 {} 失败: {:?}", branch_name, e),
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
        log::info!(
            "正在处理来自 @{} 的更新 PR #{} 请求...",
            context.requester,
            context.pr_number
        );

        // 权限检查
        let pr = match self
            .verify_pr_permission(context.pr_number, context.requester)
            .await?
        {
            Some(pr) => pr,
            None => return Ok(()),
        };

        // 找到要更新的文件
        let files = self
            .client
            .pulls(&self.owner, &self.repo)
            .list_files(context.pr_number)
            .await?
            .items;
        let ttml_file = files
            .iter()
            .find(|f| f.filename.ends_with(".ttml") && f.filename.starts_with("raw-lyrics/"));

        let file_to_update = match ttml_file {
            Some(file) => context.root_path.join(&file.filename),
            None => {
                log::error!("在 PR #{} 中未找到 .ttml 文件", context.pr_number);
                let error_comment = format!(
                    "抱歉，@{requester}，无法在此 PR 中找到需要更新的 TTML 文件。",
                    requester = context.requester
                );
                self.client
                    .issues(&self.owner, &self.repo)
                    .create_comment(context.pr_number, error_comment)
                    .await?;
                return Ok(());
            }
        };
        log::info!(
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
        log::info!("已将更新后的歌词写入到: {}", file_to_update.display());

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

        let commit_message = format!(
            "(实验性) 更新歌词文件内容\n\n由 @{} 请求更新。",
            context.requester
        );
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

        let update_comment = Self::build_body(
            &base_text,
            Some(context.original_ttml),
            Some(context.formatted_ttml),
            65535,
        );

        self.post_comment(context.pr_number, &update_comment)
            .await?;

        log::info!("成功更新 PR #{} 并发表了评论。", context.pr_number);

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

    /// 从 PR 关联的原始 Issue 中获取解析选项
    pub async fn get_options_from_original_issue(
        &self,
        pr_number: u64,
    ) -> Result<Option<OriginalIssueOptions>> {
        let pr = self
            .client
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await?;

        if let Some(issue_number) = Self::parse_issue_number_from_pr_body(pr.body.as_deref()) {
            let issue = self
                .client
                .issues(&self.owner, &self.repo)
                .get(issue_number)
                .await?;
            let body_params = Self::parse_issue_body(issue.body.as_deref().unwrap_or(""));
            let options = Self::extract_options_from_body(&body_params);
            return Ok(Some(options));
        }

        Ok(None)
    }

    /// 验证用户是否是原始 Issue 作者
    async fn verify_pr_permission(
        &self,
        pr_number: u64,
        requester: &str,
    ) -> Result<Option<octocrab::models::pulls::PullRequest>> {
        let pr = self
            .client
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await?;

        if let Some(issue_number) = Self::parse_issue_number_from_pr_body(pr.body.as_deref()) {
            let original_issue = self
                .client
                .issues(&self.owner, &self.repo)
                .get(issue_number)
                .await?;
            let original_author = &original_issue.user.login;

            if requester != original_author {
                log::warn!(
                    "@{} 尝试操作 PR #{}，但 PR 的原始作者是 @{}",
                    requester,
                    pr_number,
                    original_author
                );
                let error_comment = format!(
                    "@{requester}，你没有权限执行此操作。\n只有这个歌词提交的原始作者 (@{original_author}) 才能操作此 PR。"
                );
                self.post_comment(pr_number, &error_comment).await?;
                return Ok(None);
            }
        } else {
            log::error!("无法从 PR #{} 的正文中解析出原始 Issue 编号。", pr_number);
            let error_comment = format!(
                "@{requester}，操作失败。\n无法从此 PR 追溯到原始的歌词提交议题，因此无法验证你的权限。"
            );
            self.post_comment(pr_number, &error_comment).await?;
            return Ok(None);
        }

        Ok(Some(pr))
    }

    /// 从解析后的 Issue 正文参数中提取歌词处理选项
    pub fn extract_options_from_body(
        body_params: &HashMap<String, String>,
    ) -> OriginalIssueOptions {
        let lyric_options = body_params.get("歌词选项").cloned().unwrap_or_default();
        let advanced_toggles = body_params.get("功能开关").cloned().unwrap_or_default();
        let punctuation_weight_str = body_params
            .get("[分词] 标点符号权重")
            .cloned()
            .filter(|s| !s.is_empty() && s != "_No response_");

        OriginalIssueOptions {
            lyric_options,
            advanced_toggles,
            punctuation_weight_str,
        }
    }

    pub async fn post_pr_failure_comment(
        &self,
        pr_number: u64,
        requester: &str,
        reason: &str,
        ttml_content: &str,
    ) -> Result<()> {
        let base_text = format!(
            "@{requester}，你提交的歌词文件更新失败。\n\n**原因**: {reason}",
            requester = requester,
            reason = reason
        );

        let failure_comment = Self::build_body(&base_text, Some(ttml_content), None, 65535);

        self.post_comment(pr_number, &failure_comment).await
    }

    fn build_body(
        base_text: &str,
        original_lyric: Option<&str>,
        processed_lyric: Option<&str>,
        max_len: usize,
    ) -> String {
        const PLACEHOLDER_TEXT: &str = "```xml\n<!-- 因数据过大请自行查看变更 -->\n```";
        let separator = "\n\n";

        let original_section_title = "**原始歌词数据:**";
        let processed_section_title = "**转存歌词数据:**";

        // 尝试包含所有内容
        let body = base_text.to_string();
        let original_section = original_lyric.map(|s| {
            format!(
                "{}{}{}\n```xml\n{}\n```",
                separator, original_section_title, separator, s
            )
        });
        let processed_section = processed_lyric.map(|s| {
            format!(
                "{}{}{}\n```xml\n{}\n```",
                separator, processed_section_title, separator, s
            )
        });

        let mut final_body = body.clone();
        if let Some(ref section) = original_section {
            final_body.push_str(section);
        }
        if let Some(ref section) = processed_section {
            final_body.push_str(section);
        }

        if final_body.len() <= max_len {
            return final_body;
        }

        // 如果超长，尝试只包含处理后的歌词
        if let Some(ref section) = processed_section {
            let mut final_body = body.clone();
            let placeholder_original = format!(
                "{}{}{}{}",
                separator, original_section_title, separator, PLACEHOLDER_TEXT
            );

            final_body.push_str(&placeholder_original);
            final_body.push_str(section);
            if final_body.len() <= max_len {
                return final_body;
            }
        }

        // 如果仍然超长，对所有歌词都使用占位符
        let mut final_body = body.clone();
        if original_lyric.is_some() {
            let placeholder_original = format!(
                "{}{}{}{}",
                separator, original_section_title, separator, PLACEHOLDER_TEXT
            );

            final_body.push_str(&placeholder_original);
        }
        if processed_lyric.is_some() {
            let placeholder_processed = format!(
                "{}{}{}",
                separator, processed_section_title, PLACEHOLDER_TEXT
            );
            final_body.push_str(&placeholder_processed);
        }

        // 如果连占位符都放不下，就只返回基础文本
        if final_body.len() <= max_len {
            final_body
        } else {
            body
        }
    }

    // 构建在 Issue 中发表的成功评论
    fn build_issue_success_comment(
        original_lyric: &str,
        processed_lyric: &str,
        warnings: &[String],
    ) -> String {
        let mut base_text = format!(
            "{}\n\n歌词提交议题检查完毕！\n已自动创建歌词提交合并请求！\n请耐心等待管理员审核歌词吧！",
            CHECKED_MARK
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

        Self::build_body(
            &base_text,
            Some(original_lyric),
            Some(processed_lyric),
            65535,
        )
    }
}
