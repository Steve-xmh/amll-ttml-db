import fetch from "node-fetch";
import { execSync } from "child_process";
import { Octokit } from "octokit";
import { parseLyric } from "./ttml-parser.js";
import { exportTTMLText } from "./ttml-writer.js";
import { writeFile } from "fs/promises";
import { resolve } from "path";

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

function parseBody(body) {
	const params = {};
	let curKey = "";
	for (const line of body) {
		if (line.startsWith("### ")) {
			curKey = line.substring(4).trim();
			params[curKey] = "";
		} else if (line.startsWith("```")) {
			continue;
		} else {
			params[curKey] += line + "\n";
		}
	}
	return params;
}

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
			const hasPullRequest = (await octokit.rest.search.issuesAndPullRequests({
				q: `repo:${REPO_OWNER}/${REPO_NAME} is:pr author:app/github-actions 歌词议题 #${issue.number}`,
			})).data.total_count > 0;
			if (hasPullRequest) {
				console.log("议题", issue.title, "(", issue.id, ") 已存在关联的合并请求，跳过");
				continue;
			}
			async function confirmIssue(lyric, regeneratedLyric) {
				let commentBody = [
					HAS_CHECKED_MARK,
					"歌词提交议题检查完毕！歌词文件没有异常！",
					"已自动创建歌词提交合并请求！",
					"请耐心等待管理员审核歌词吧！",
					"以下是留存的歌词数据：",
					"```xml",
					...lyric.split("\n"),
					"```",
					"以下是转存的歌词数据：",
					"```xml",
					...regeneratedLyric.split("\n"),
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
						...regeneratedLyric.split("\n"),
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
				const musicIds = String(params["音乐对应的网易云音乐 ID"])
					.split(",")
					.map((v) => parseInt(v.trim()))
					.filter((v) => v > 0 && Number.isSafeInteger(v));
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
				if (musicIds.length === 0) {
					console.log(
						"议题",
						issue.title,
						"(",
						issue.id,
						") 无法识别到对应的网易云音乐 ID",
					);
					await declineIssue("无法识别到对应的网易云音乐 ID");
					continue;
				}
				console.log("正在下载 TTML 歌词文件", lyricURL.trim());
				try {
					const lyric = await fetch(lyricURL).then((v) => v.text());
					try {
						const parsedLyric = parseLyric(lyric);
						const regeneratedLyric = await exportTTMLText(parsedLyric);
						const regeneratedLyricFormatted = await exportTTMLText(
							parsedLyric,
							true,
						);
						await confirmIssue(lyric, regeneratedLyric);
						console.log(
							"议题",
							issue.title,
							"(",
							issue.id,
							") 检查完毕！正在创建合并请求……",
						);
						try {
							const submitBranch = "auto-submit-issue-" + issue.number;
							execSync("git checkout main");
							try {
								execSync("git branch -D " + submitBranch);
							} catch {}
							execSync("git checkout --force -b " + submitBranch);
							await Promise.all(
								musicIds.map(async (v) => {
									await writeFile(resolve("..", "lyrics", `${v}.ttml`), lyric);
								}),
							);
							execSync("git add ..");
							execSync(`git commit -m "提交歌曲歌词 #${issue.number}"`);
							execSync("git push --set-upstream origin " + submitBranch);
							execSync("git checkout main");
							let pullBody = [
								"### 歌词议题",
								"#" + issue.number,
								"### 歌词文件内容",
								"```xml",
								regeneratedLyric,
								"```",
								"### 歌词文件内容（已格式化）",
								"```xml",
								regeneratedLyricFormatted,
								"```",
							].join("\n");
							if (pullBody.length > 65536) {
								pullBody = [
									"### 歌词议题",
									"#" + issue.number,
									"### 歌词文件内容",
									"```xml",
									"<!-- 因数据过大请自行查看变更 -->",
									"```",
									"### 歌词文件内容（已格式化）",
									"```xml",
									regeneratedLyricFormatted,
									"```",
								].join("\n");
								if (pullBody.length > 65536) {
									pullBody = [
										"### 歌词议题",
										"#" + issue.number,
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
							console.log(
								"议题",
								issue.title,
								"(",
								issue.id,
								") 关联合并请求创建成功！",
							);
						} catch (err) {
							console.log(
								"议题",
								issue.title,
								"(",
								issue.id,
								") 创建合并请求失败！请手动提交！",
								err,
							);
						}
					} catch (err) {
						console.log(
							"议题",
							issue.title,
							"(",
							issue.id,
							") 解析 TTML 歌词文件失败",
							err,
						);
						await declineIssue("解析 TTML 歌词文件失败", err);
					}
				} catch (err) {
					console.log(
						"议题",
						issue.title,
						"(",
						issue.id,
						") 下载 TTML 歌词文件失败",
						err,
					);
					await declineIssue("下载 TTML 歌词文件失败", err);
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
		}
	}
	console.log("检查完毕");
}
main().catch(console.error);
