use anyhow::{Result, anyhow};
use std::{path::Path, process::Stdio};
use tokio::process::Command;
use tracing::{error, info};

async fn run_git_command(args: &[&str]) -> Result<()> {
    info!("正在执行 Git 命令: git {}", args.join(" "));
    let output = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        info!("Git 命令成功执行。");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!(
            "Git 命令 `git {}` 失败: {}",
            args.join(" "),
            stderr
        ))
    }
}

pub async fn checkout_main_branch() -> Result<()> {
    run_git_command(&["checkout", "main"]).await?;
    run_git_command(&["pull"]).await
}

pub async fn create_branch(branch_name: &str) -> Result<()> {
    run_git_command(&["checkout", "-b", branch_name]).await
}

pub async fn commit(message: &str) -> Result<()> {
    run_git_command(&["commit", "-m", message]).await
}

pub async fn push(branch_name: &str) -> Result<()> {
    run_git_command(&["push", "--set-upstream", "origin", branch_name]).await
}

pub async fn add_path(path_to_add: &Path) -> Result<()> {
    let path_str = path_to_add
        .to_str()
        .ok_or_else(|| anyhow!("路径 {} 包含无效的 UTF-8 字符", path_to_add.display()))?;

    run_git_command(&["add", path_str]).await
}

pub async fn delete_branch_if_exists(branch_name: &str) -> Result<()> {
    match run_git_command(&["branch", "-D", branch_name]).await {
        Ok(()) => {
            info!("成功删除了分支: {branch_name}");
        }
        Err(_) => {
            info!("无法删除分支 '{branch_name}'，可能它不存在。");
        }
    }
    Ok(())
}

pub async fn checkout_branch(branch_name: &str) -> Result<()> {
    run_git_command(&["checkout", branch_name]).await
}

pub async fn pull_branch(branch_name: &str) -> Result<()> {
    run_git_command(&["pull", "origin", branch_name]).await
}

pub async fn force_push(branch_name: &str) -> Result<()> {
    run_git_command(&["push", "--force", "origin", branch_name]).await
}

pub async fn has_staged_changes() -> Result<bool> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .output()
        .await?;

    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("检查暂存区变更时出错: {stderr}");
            Err(anyhow!(
                "Git 命令 `git diff --cached --quiet` 失败: {stderr}"
            ))
        }
    }
}
