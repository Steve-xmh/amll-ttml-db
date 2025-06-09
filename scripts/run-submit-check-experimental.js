import fetch from "node-fetch";
import { uid } from "uid/secure";
import { execSync } from "child_process";
import { Octokit } from "octokit";
import { writeFile, readFile, unlink } from "fs/promises";
import { resolve } from "path";
import prettier from "prettier";
import {
  HAS_CHECKED_MARK,
  REPO_NAME,
  REPO_OWNER,
  addFileToGit,
  checkoutBranch,
  commit,
  createBranch,
  deleteBranch,
  githubToken,
  parseBody,
  push,
} from "./utils.js";
import path from "path";
import { fileURLToPath } from "url";

const octokit = new Octokit({
  auth: githubToken,
  userAgent: "AMLLTTMLDBSubmitChecker",
});

async function main() {
  const openingIssues = await octokit.rest.issues.listForRepo({
    owner: REPO_OWNER,
    repo: REPO_NAME,
    state: "open",
    labels: "实验性歌词提交/修正",
  });

  for (const issue of openingIssues.data) {
    const tempInputFile = `temp-input-${issue.id}.ttml`;
    const tempProcessedFile = `temp-processed-${issue.id}.ttml`;
    const tempMetadataFile = `temp-metadata-${issue.id}.json`;

    try {
      console.log("正在检查议题 (实验性流程)", issue.title, "(", issue.id, ")");
      const comments = await octokit.rest.issues.listComments({
        owner: REPO_OWNER,
        repo: REPO_NAME,
        issue_number: issue.number,
      });
      const hasPullRequest =
        (
          await octokit.rest.search.issuesAndPullRequests({
            q: `repo:${REPO_OWNER}/${REPO_NAME} is:pr author:app/github-actions head:auto-submit-issue-${issue.number}`,
          })
        ).data.total_count > 0;

      if (hasPullRequest) {
        console.log(
          "议题",
          issue.title,
          "(",
          issue.id,
          ") 已存在关联的合并请求，跳过",
        );
        continue;
      }
      async function confirmIssue(originalLyric, processedLyric) {
        let commentBody = [
          HAS_CHECKED_MARK,
          "歌词提交议题检查完毕！歌词文件没有异常！",
          "已自动创建歌词提交合并请求！",
          "请耐心等待管理员审核歌词吧！",
          "以下是留存的歌词数据：",
          "```xml",
          ...originalLyric.split("\n"),
          "```",
          "以下是转存的歌词数据：",
          "```xml",
          ...processedLyric.split("\n"),
          "```",
        ].join("\n");
        if (commentBody.length > 65536) {
          commentBody = [
            HAS_CHECKED_MARK,
            "歌词提交议题检查完毕！歌词文件没有异常！",
            "已自动创建歌词提交合并请求！",
            "请耐心等待管理员审核歌词吧！",
            "以下是留存的歌词数据：",
            "```xml",
            "<!-- 因数据过大不做留存 -->",
            "```",
            "以下是转存的歌词数据：",
            "```xml",
            ...processedLyric.split("\n"),
            "```",
          ].join("\n");
          if (commentBody.length > 65536) {
            commentBody = [
              HAS_CHECKED_MARK,
              "歌词提交议题检查完毕！歌词文件没有异常！",
              "已自动创建歌词提交合并请求！",
              "请耐心等待管理员审核歌词吧！",
              "以下是留存的歌词数据：",
              "```xml",
              "<!-- 因数据过大不做留存 -->",
              "```",
              "以下是转存的歌词数据：",
              "```xml",
              "<!-- 因数据过大不做转存显示 -->",
              "```",
            ].join("\n");
          }
        }

        await octokit.rest.issues.createComment({
          owner: REPO_OWNER,
          repo: REPO_NAME,
          issue_number: issue.number,
          body: commentBody,
        });
        await octokit.rest.issues.update({
          owner: REPO_OWNER,
          repo: REPO_NAME,
          issue_number: issue.number,
          state: "closed",
          state_reason: "completed",
        });
        await octokit.rest.issues.lock({
          owner: REPO_OWNER,
          repo: REPO_NAME,
          issue_number: issue.number,
          lock_reason: "resolved",
        });
      }
      async function declineIssue(msg, err = null, lyric = "") {
        await octokit.rest.issues.createComment({
          owner: REPO_OWNER,
          repo: REPO_NAME,
          issue_number: issue.number,
          body: [
            HAS_CHECKED_MARK,
            "歌词提交议题检查失败：" + msg,
            ...(err && String(err).trim().length > 0
              ? ["详细错误输出：", "```", ...String(err).split("\n"), "```"]
              : []),
            ...(lyric.trim().length > 0
              ? ["获取到的歌词数据：", "```xml", ...lyric.split("\n"), "```"]
              : []),
          ].join("\n"),
        });
        await octokit.rest.issues.update({
          owner: REPO_OWNER,
          repo: REPO_NAME,
          issue_number: issue.number,
          state: "closed",
          state_reason: "completed",
        });
        await octokit.rest.issues.lock({
          owner: REPO_OWNER,
          repo: REPO_NAME,
          issue_number: issue.number,
          lock_reason: "resolved",
        });
      }
      if (
        comments.data.find(
          (v) =>
            (v.user?.type === "Bot" || v.user?.id === 39523898) &&
            v.body?.startsWith(HAS_CHECKED_MARK),
        )
      ) {
        console.log("议题", issue.title, "(", issue.id, ") 已检查，跳过");
        continue;
      } else {
        console.log(
          "议题",
          issue.title,
          "(",
          issue.id,
          ") 尚未开始检查，正在搜索直链",
        );
        const body = issue.body?.split("\n")?.filter((v) => v.length > 0) ?? [];
        const params = parseBody(body);
        const lyricURL = params["TTML 歌词文件下载直链"];
        const comment = params["备注"].trim().split("\n");

        // 获取歌词选项
        const lyricOptions = params["歌词选项"] || "";

        // 判断复选框是否被勾选
        const isLineTimedByUser = lyricOptions.includes("[x]");
        const timingMode = isLineTimedByUser ? "line" : "word";

        console.log(`用户指定的计时模式: ${timingMode}`);

        if (typeof lyricURL !== "string") {
          console.log(
            "议题",
            issue.title,
            "(",
            issue.id,
            ") 无法找到 TTML 歌词文件下载直链",
          );

          await declineIssue("无法找到 TTML 歌词文件下载直链");
          continue;
        }

        console.log("正在下载 TTML 歌词文件", lyricURL.trim());
        const originalLyricContent = await fetch(lyricURL).then((v) => {
          if (v.ok) return v.text();
          else throw new Error(v.statusText);
        });

        await writeFile(tempInputFile, originalLyricContent);

        try {
          console.log("正在调用 Rust 处理器...");

          const __filename = fileURLToPath(import.meta.url);
          const __dirname = path.dirname(__filename);

          const rustProcessorPath = path.resolve(
            __dirname,
            "..",
            "target",
            "release",
            "ttml_processor",
          );
          const command = `"${rustProcessorPath}" --input ${tempInputFile} --output ${tempProcessedFile} --json-output ${tempMetadataFile} --timing-mode ${timingMode}`;
          execSync(command, { stdio: "inherit" });
          console.log("Rust 处理器执行成功，文件已验证并生成。");

          const regeneratedLyric = await readFile(tempProcessedFile, "utf-8");
          const metadataJson = await readFile(tempMetadataFile, "utf-8");
          const metadata = JSON.parse(metadataJson);

          const lyricFormatted = await prettier.format(regeneratedLyric, {
            parser: "html",
          });

          function buildMetadataMessage(metadata) {
            const musicPlatformKeyLabelPairs = {
              NcmMusicId: "歌曲关联网易云音乐 ID",
              QqMusicId: "歌曲关联 QQ 音乐 ID",
              SpotifyId: "歌曲关联 Spotify 音乐 ID",
              AppleMusicId: "歌曲关联 Apple Music 音乐 ID",
            };
            let containsId = false;
            const pullMetadataMessage = [];
            const musicName = metadata.Title || [];
            const artists = metadata.Artist || [];
            const album = metadata.Album || [];

            let addToolsUsageTip = false;

            if (musicName.length === 0) {
              pullMetadataMessage.push(
                "歌词文件中未包含歌曲名称信息（缺失 Title 元数据）",
              );
              addToolsUsageTip = true;
            }
            if (artists.length === 0) {
              pullMetadataMessage.push(
                "歌词文件中未包含音乐作者信息（缺失 Artist 元数据）",
              );
              addToolsUsageTip = true;
            }
            if (album.length === 0) {
              pullMetadataMessage.push(
                "歌词文件中未包含专辑信息（缺失 Album 元数据）(注：如果是单曲专辑请和歌曲名称同名)",
              );
              addToolsUsageTip = true;
            }

            pullMetadataMessage.push("### 音乐名称");
            musicName.forEach((v) => pullMetadataMessage.push(`- \`${v}\``));
            pullMetadataMessage.push("### 音乐作者");
            artists.forEach((v) => pullMetadataMessage.push(`- \`${v}\``));
            pullMetadataMessage.push("### 音乐专辑名称");
            album.forEach((v) => pullMetadataMessage.push(`- \`${v}\``));

            for (const key in musicPlatformKeyLabelPairs) {
              const ids = metadata[key];
              if (ids && ids.length > 0) {
                containsId = true;
                pullMetadataMessage.push(
                  `### ${musicPlatformKeyLabelPairs[key]}`,
                );
                for (const id of ids) {
                  pullMetadataMessage.push(`- \`${id}\``);
                }
              }
            }
            if (!containsId) {
              pullMetadataMessage.push("歌词文件中未包含任何音乐平台 ID");
              addToolsUsageTip = true;
            }
            if (addToolsUsageTip) {
              pullMetadataMessage.push(
                "（注：如果你正在使用 AMLL TTML Tools 歌词编辑工具，可以通过顶部菜单 编辑 - 编辑歌曲元数据 来编辑元数据）",
              );
            }

            return pullMetadataMessage;
          }
          const metadataMessage = buildMetadataMessage(metadata);
          await confirmIssue(originalLyricContent, regeneratedLyric);
          const submitBranch = "auto-submit-issue-" + issue.number;
          await checkoutBranch("main");
          try {
            await deleteBranch(submitBranch);
          } catch {}
          await createBranch(submitBranch);
          const newFileName = `${Date.now()}-${issue.user?.id || "0"}-${uid(
            8,
          )}.ttml`;
          await writeFile(
            resolve("..", "raw-lyrics", newFileName),
            regeneratedLyric,
          );
          await addFileToGit("..");
          await commit(`(实验性) 提交歌曲歌词 ${newFileName} #${issue.number}`);
          await push(submitBranch);
          await checkoutBranch("main");
          let pullBody = [
            `### 歌词议题 (实验性流程)`,
            `#${issue.number}`,
            `### 歌词作者`,
            issue.user?.login
              ? `@${issue.user?.login}`
              : "未知，请查看议题发送者",
            `---`,
            ...metadataMessage,
            `### 备注`,
            ...comment,
            `### 歌词文件内容`,
            "```xml",
            originalLyricContent,
            "```",
            `### 歌词文件内容（已格式化）`,
            "```xml",
            lyricFormatted,
            "```",
          ].join("\n");

          if (pullBody.length > 65536) {
            pullBody = [
              "### 歌词议题 (实验性流程)",
              "#" + issue.number,
              "### 歌词作者",
              issue.user?.login
                ? "@" + issue.user?.login
                : "未知，请查看议题发送者",
              ...metadataMessage,
              "### 备注",
              ...comment,
              "### 歌词文件内容",
              "```xml",
              "<!-- 因数据过大请自行查看变更 -->",
              "```",
              "### 歌词文件内容（已格式化）",
              "```xml",
              lyricFormatted,
              "```",
            ].join("\n");
            if (pullBody.length > 65536) {
              pullBody = [
                "### 歌词议题 (实验性流程)",
                "#" + issue.number,
                "### 歌词作者",
                issue.user?.login
                  ? "@" + issue.user?.login
                  : "未知，请查看议题发送者",
                ...metadataMessage,
                "### 歌词文件内容",
                "```xml",
                "<!-- 因数据过大请自行查看变更 -->",
                "```",
                "### 歌词文件内容（已格式化）",
                "```xml",
                "<!-- 因数据过大请自行查看变更 -->",
                "```",
              ].join("\n");
            }
          }

          await octokit.rest.pulls.create({
            owner: REPO_OWNER,
            repo: REPO_NAME,
            title: issue.title,
            head: submitBranch,
            base: "main",
            body: pullBody,
          });
        } catch (error) {
          console.error("Rust 处理器执行失败:", error);
          const errorMessages = error.stderr
            ? error.stderr.toString()
            : "无法获取 Rust 处理器的具体错误信息。";
          await declineIssue(
            "Rust 处理器执行失败。",
            errorMessages,
            originalLyricContent,
          );
          continue;
        }
      }
    } catch (err) {
      console.warn(
        "检查议题",
        issue.title,
        "(",
        issue.id,
        ") 时发生意料之外的错误",
        err,
      );
    } finally {
      try {
        await unlink(tempInputFile);
      } catch {}
      try {
        await unlink(tempProcessedFile);
      } catch {}
      try {
        await unlink(tempMetadataFile);
      } catch {}
    }
  }
  console.log("检查完毕");
}

main().catch(console.error);
