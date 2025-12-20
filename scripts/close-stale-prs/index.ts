/** biome-ignore-all lint/complexity/useLiteralKeys: 与 ts 配置 noPropertyAccessFromIndexSignature 冲突 */

import { Octokit } from "octokit";

const LABEL_NAME = "待更新";
const DAYS_THRESHOLD = 3;

const repoEnv = process.env["GITHUB_REPOSITORY"] || "";
const OWNER = process.env["OWNER"] || repoEnv.split("/")[0] || "Steve-xmh";
const REPO = process.env["REPO"] || repoEnv.split("/")[1] || "amll-ttml-db";
const TOKEN = process.env["GITHUB_TOKEN"];

const IS_DRY_RUN = process.env["DRY_RUN"] === "true";

if (!OWNER || !REPO || !TOKEN) {
	console.error(
		"缺少环境变量 GITHUB_TOKEN 以及 GITHUB_REPOSITORY 或 OWNER/REPO",
	);
	process.exit(1);
}

interface LabeledEvent {
	event: "labeled";
	created_at: string;
	label: {
		name: string;
	};
}

function isLabeledEvent(event: unknown): event is LabeledEvent {
	if (typeof event !== "object" || event === null) {
		return false;
	}

	const e = event as Record<string, unknown>;

	return (
		e["event"] === "labeled" &&
		typeof e["label"] === "object" &&
		e["label"] !== null &&
		"name" in e["label"]
	);
}

const octokit = new Octokit({ auth: TOKEN });

async function run() {
	if (IS_DRY_RUN) {
		console.log("[DRY RUN] 模拟运行模式，将不会执行任何操作");
	}

	const prs = await octokit.paginate(octokit.rest.pulls.list, {
		owner: OWNER,
		repo: REPO,
		state: "open",
		per_page: 100,
	});

	const targetPrs = prs.filter((pr) =>
		pr.labels.some((label) => label.name === LABEL_NAME),
	);

	console.log(`[i] 找到 ${targetPrs.length} 个待检查的 PR`);

	for (const pr of targetPrs) {
		console.log(`-> 检查 PR ${pr.number}: ${pr.title}`);

		try {
			// 标签添加的时间
			const events = await octokit.paginate(octokit.rest.issues.listEvents, {
				owner: OWNER,
				repo: REPO,
				issue_number: pr.number,
				per_page: 100,
			});

			const labelEvent = events
				.reverse()
				.find((e) => isLabeledEvent(e) && e.label.name === LABEL_NAME);

			if (!labelEvent || !labelEvent.created_at) {
				console.log(`找不到标签添加时间，跳过`);
				continue;
			}

			// 最后一次 commit 的时间
			const { data: latestCommit } = await octokit.rest.repos.getCommit({
				owner: OWNER,
				repo: REPO,
				ref: pr.head.sha,
			});

			const lastCommitDateStr =
				latestCommit.commit.committer?.date || latestCommit.commit.author?.date;

			if (!lastCommitDateStr) {
				console.log(`找不到 Commit 时间，跳过`);
				continue;
			}

			// 时间差
			const now = new Date();
			const labelDate = new Date(labelEvent.created_at);
			const commitDate = new Date(lastCommitDateStr);

			const daysSinceLabel =
				(now.getTime() - labelDate.getTime()) / (1000 * 3600 * 24);
			const daysSinceCommit =
				(now.getTime() - commitDate.getTime()) / (1000 * 3600 * 24);

			console.log(`    标签添加距今 ${daysSinceLabel.toFixed(1)} 天`);
			console.log(`    最后提交距今 ${daysSinceCommit.toFixed(1)} 天`);

			if (daysSinceLabel > DAYS_THRESHOLD && daysSinceCommit > DAYS_THRESHOLD) {
				const branchName = pr.head.ref;
				const isSameRepo = pr.head.repo?.full_name === `${OWNER}/${REPO}`;
				const shouldDeleteBranch =
					isSameRepo && branchName.startsWith("auto-submit-issue");

				if (IS_DRY_RUN) {
					console.log(`[!] 满足关闭条件`);
					console.log(`    拟添加评论并关闭 PR #${pr.number}`);
				} else {
					console.log(`[x] 满足条件，正在关闭此 PR`);

					await octokit.rest.issues.createComment({
						owner: OWNER,
						repo: REPO,
						issue_number: pr.number,
						body: `你好，由于此 PR 需要更新，但超过 ${DAYS_THRESHOLD} 天未更新，我们已关闭此 PR。如需更新歌词，请打开一个新的 PR。`,
					});

					await octokit.rest.pulls.update({
						owner: OWNER,
						repo: REPO,
						pull_number: pr.number,
						state: "closed",
					});

					if (shouldDeleteBranch) {
						try {
							console.log(`    删除分支 "${branchName}"`);
							await octokit.rest.git.deleteRef({
								owner: OWNER,
								repo: REPO,
								ref: `heads/${branchName}`,
							});
						} catch (err) {
							console.error(`    删除分支失败`, err);
						}
					}
				}
			} else {
				console.log(`    不满足条件，跳过`);
			}
		} catch (error) {
			console.error(`处理 PR #${pr.number} 时出错`, error);
		}
	}
}

run();
