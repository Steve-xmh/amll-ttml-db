/** biome-ignore-all lint/complexity/useLiteralKeys: 与 ts 配置 noPropertyAccessFromIndexSignature 冲突 */

import { Octokit } from "octokit";

const LABEL_NAME = "待更新";
const DAYS_THRESHOLD = 7;

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

const octokit = new Octokit({ auth: TOKEN });

async function getLastCommitTime(
	owner: string,
	repo: string,
	commitSha: string,
): Promise<number> {
	try {
		const { data: commit } = await octokit.rest.repos.getCommit({
			owner,
			repo,
			ref: commitSha,
		});

		const dateStr = commit.commit.committer?.date || commit.commit.author?.date;
		return dateStr ? new Date(dateStr).getTime() : 0;
	} catch (error) {
		console.error(`    💥 获取 Commit 详情失败 (SHA: ${commitSha})`, error);
		return 0;
	}
}

async function run() {
	if (IS_DRY_RUN) {
		console.log("🧪 [DRY RUN] 模拟运行模式，将不会执行任何操作");
	}

	console.log("🔍 正在获取 Open 状态的 PR...");
	const prs = await octokit.paginate(octokit.rest.pulls.list, {
		owner: OWNER,
		repo: REPO,
		state: "open",
		per_page: 100,
	});

	const now = Date.now();
	const msPerDay = 1000 * 3600 * 24;

	const stalePrs = prs.filter((pr) => {
		const updatedAt = new Date(pr.updated_at).getTime();
		return (now - updatedAt) / msPerDay > DAYS_THRESHOLD;
	});

	console.log(
		`🔍 总 Open PR: ${prs.length}。其中 ${stalePrs.length} 个 PR 超过 ${DAYS_THRESHOLD} 天未活跃 (无代码、无评论、无状态变更)。`,
	);

	for (const pr of stalePrs) {
		console.log(`\n📋 检查 PR #${pr.number}: ${pr.title}`);

		try {
			let shouldClose = false;
			let closeReason = "";

			const currentLabelNames = pr.labels.map((l) => l.name);
			const hasWaitingLabel = currentLabelNames.includes(LABEL_NAME);

			if (hasWaitingLabel) {
				shouldClose = true;
				closeReason = `🏷️ 存在 "${LABEL_NAME}" 标签且 ${DAYS_THRESHOLD} 天无任何活跃`;
			} else {
				const reviews = await octokit.paginate(octokit.rest.pulls.listReviews, {
					owner: OWNER,
					repo: REPO,
					pull_number: pr.number,
					per_page: 100,
				});

				const lastChangeRequest = reviews
					.reverse()
					.find((review) => review.state === "CHANGES_REQUESTED");

				if (lastChangeRequest) {
					const reviewTime = new Date(
						lastChangeRequest.submitted_at || "",
					).getTime();
					const lastCommitTime = await getLastCommitTime(
						OWNER,
						REPO,
						pr.head.sha,
					);

					if (reviewTime > lastCommitTime) {
						shouldClose = true;
						closeReason = `📝 Review 请求更改后，${DAYS_THRESHOLD} 天无新代码提交或活跃`;
					} else {
						console.log(
							`    ✋ 用户已提交新代码 (Commit 于 Review 之后)，等待管理员审核，跳过。`,
						);
					}
				}
			}

			if (shouldClose) {
				const daysSinceUpdate = (
					(now - new Date(pr.updated_at).getTime()) /
					msPerDay
				).toFixed(1);
				console.log(`    🚫 满足关闭条件: ${closeReason}`);
				console.log(`    ⏳ 最后活跃距今: ${daysSinceUpdate} 天`);

				const branchName = pr.head.ref;
				const isSameRepo = pr.head.repo?.full_name === `${OWNER}/${REPO}`;
				const shouldDeleteBranch =
					isSameRepo && branchName.startsWith("auto-submit-issue");

				if (IS_DRY_RUN) {
					console.log(`    🔔 [DRY RUN] 满足关闭条件`);
					console.log(`        拟添加评论并关闭 PR #${pr.number}`);
				} else {
					console.log(`    🚫 满足条件，正在关闭此 PR...`);

					await octokit.rest.issues.createComment({
						owner: OWNER,
						repo: REPO,
						issue_number: pr.number,
						body: `你好，由于此 PR 当前处于待修改状态，且超过 ${DAYS_THRESHOLD} 天没有任何更新，我们已将其关闭。如需继续贡献，请重新打开一个新的 PR。`,
					});

					await octokit.rest.pulls.update({
						owner: OWNER,
						repo: REPO,
						pull_number: pr.number,
						state: "closed",
					});

					if (shouldDeleteBranch) {
						try {
							console.log(`    🗑️ 删除分支 "${branchName}"`);
							await octokit.rest.git.deleteRef({
								owner: OWNER,
								repo: REPO,
								ref: `heads/${branchName}`,
							});
						} catch (err) {
							console.error(`    💥 删除分支失败`, err);
						}
					}
				}
			} else if (!hasWaitingLabel) {
				console.log(`    ✅ PR 既无待更新标签，也无阻塞的 Review，跳过。`);
			}
		} catch (error) {
			console.error(`💥 处理 PR #${pr.number} 时出错`, error);
		}
	}
}

run();
