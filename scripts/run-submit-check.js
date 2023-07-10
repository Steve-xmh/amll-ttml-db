import fetch from "node-fetch";
import { Octokit } from "octokit";
import { parseLyric } from "./ttml-parser.js";
import { exportTTMLText } from "./ttml-writer.js";

const githubToken = process.env.GITHUB_TOKEN;
const octokit = new Octokit({
  auth: githubToken,
  userAgent: "AMLLTTMLDBSubmitChecker",
});
const [REPO_OWNER, REPO_NAME] = process.env.GITHUB_REPOSITORY?.split("/") ?? [
  "Steve-xmh",
  "amll-ttml-db",
];

const HAS_CHECKED_MARK = "<!-- AMLL-DB-BOT-CHECKED -->";

async function main() {
  const openingIssues = await octokit.rest.issues.listForRepo({
    owner: REPO_OWNER,
    repo: REPO_NAME,
    state: "open",
    labels: "歌词提交/补正",
  });
  for (const issue of openingIssues.data) {
    try {
      console.log("正在检查议题", issue.title, "(", issue.id, ")");
      const comments = await octokit.rest.issues.listComments({
        owner: REPO_OWNER,
        repo: REPO_NAME,
        issue_number: issue.number,
      });
      if (
        comments.data.find(
          (v) =>
            (v.user?.type === "Bot" || v.user?.id === 39523898) &&
            v.body?.startsWith(HAS_CHECKED_MARK)
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
          ") 尚未开始检查，正在搜索直链"
        );
        const body = issue.body?.split("\n")?.filter((v) => v.length > 0) ?? [];
        let curKey = "";
        const params = {};
        for (const line of body) {
          if (line.startsWith("### ")) {
            curKey = line.substring(4).trim();
            params[curKey] = "";
          } else {
            params[curKey] += line + "\n";
          }
        }
        const lyricURL = params["TTML 歌词文件下载直链"];
        if (typeof lyricURL === "string") {
          console.log("正在下载 TTML 歌词文件", lyricURL.trim());
          const lyric = await fetch(lyricURL).then((v) => v.text());
          try {
            const parsedLyric = parseLyric(lyric);
            const regeneratedLyric = exportTTMLText(parsedLyric);
            await octokit.rest.issues.createComment({
              owner: REPO_OWNER,
              repo: REPO_NAME,
              issue_number: issue.number,
              body: [
                HAS_CHECKED_MARK,
                "歌词提交议题检查完毕！歌词文件没有异常！",
                "请耐心等待管理员审核歌词吧！",
                "以下是留存的歌词数据：",
                "```",
                ...lyric.split("\n"),
                "```",
                "以下是转存的歌词数据：",
                "```",
                ...regeneratedLyric.split("\n"),
                "```",
              ].join("\n"),
            });
            console.log("议题", issue.title, "(", issue.id, ") 检查完毕！");
          } catch (err) {
            console.log(
              "议题",
              issue.title,
              "(",
              issue.id,
              ") 解析 TTML 歌词文件失败",
              err
            );
            await octokit.rest.issues.update({
              owner: REPO_OWNER,
              repo: REPO_NAME,
              issue_number: issue.number,
              state: "closed",
              state_reason: "not_planned",
              body: [
                HAS_CHECKED_MARK,
                "歌词提交议题检查失败：解析 TTML 歌词文件失败",
                "详细错误输出：",
                "```",
                ...String(err).split("\n"),
                "```",
                "获取到的歌词文件数据：",
                "```",
                ...lyric.split("\n"),
                "```",
              ].join("\n"),
            });
          }
        } else {
          console.log(
            "议题",
            issue.title,
            "(",
            issue.id,
            ") 无法找到 TTML 歌词文件下载直链"
          );
          octokit.rest.issues.update({
            owner: REPO_OWNER,
            repo: REPO_NAME,
            issue_number: issue.number,
            state: "closed",
            state_reason: "not_planned",
            body: [
              HAS_CHECKED_MARK,
              "歌词提交议题检查失败：无法找到 TTML 歌词文件下载直链",
            ].join("\n"),
          });
        }
      }
    } catch (err) {
      console.warn(
        "检查议题",
        issue.title,
        "(",
        issue.id,
        ") 时发生意料之外的错误",
        err
      );
    }
  }
  console.log("检查完毕");
}
main().catch(console.error);
