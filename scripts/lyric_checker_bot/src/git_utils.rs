use anyhow::{Result, anyhow};
use std::{path::Path, process::Stdio};
use tokio::process::Command;

/// 运行一个 Git 命令并等待其完成，如果失败则返回错误。
async fn run_git_command(args: &[&str]) -> Result<()> {
    log::info!("正在执行 Git 命令: git {}", args.join(" "));
    let output = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        log::info!("Git 命令成功执行。");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("Git 命令执行失败: {stderr}");
        Err(anyhow!(
            "Git 命令 `git {}` 失败: {}",
            args.join(" "),
            stderr
        ))
    }
}

/// 检出分支
pub async fn checkout_main_branch() -> Result<()> {
    // 尝试 checkout main，如果失败再尝试 master
    if run_git_command(&["checkout", "main"]).await.is_err() {
        log::warn!("检出 'main' 分支失败，尝试检出 'master'...");
        run_git_command(&["checkout", "master"]).await?;
    }
    // 拉取最新代码
    run_git_command(&["pull"]).await
}

/// 创建并切换到新分支
pub async fn create_branch(branch_name: &str) -> Result<()> {
    run_git_command(&["checkout", "-b", branch_name]).await
}

/// 提交
pub async fn commit(message: &str) -> Result<()> {
    run_git_command(&["commit", "-m", message]).await
}

/// 推送
pub async fn push(branch_name: &str) -> Result<()> {
    run_git_command(&["push", "--set-upstream", "origin", branch_name]).await
}

pub async fn add_path(path_to_add: &Path) -> Result<()> {
    let path_str = path_to_add
        .to_str()
        .ok_or_else(|| anyhow!("路径 {:?} 包含无效的 UTF-8 字符", path_to_add))?;

    run_git_command(&["add", path_str]).await
}

pub async fn delete_branch_if_exists(branch_name: &str) -> Result<()> {
    match run_git_command(&["branch", "-D", branch_name]).await {
        Ok(()) => {
            log::info!("成功删除了分支: {branch_name}");
        }
        Err(_) => {
            log::info!("无法删除分支 '{branch_name}'，可能它不存在。");
        }
    }
    Ok(())
}
