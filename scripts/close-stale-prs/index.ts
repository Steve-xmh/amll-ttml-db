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
		console.log("🧪 [DRY RUN] 模拟运行模式，将不会执行任何操作");
	}

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
		`🔍 找到 ${stalePrs.length} 个超过 ${DAYS_THRESHOLD} 天未更新的 PR。总 Open PR: ${prs.length}`,
	);

	for (const pr of stalePrs) {
		console.log(`\n📋 检查 PR #${pr.number}: ${pr.title}`);

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

			const reviews = await octokit.paginate(octokit.rest.pulls.listReviews, {
				owner: OWNER,
				repo: REPO,
				pull_number: pr.number,
				per_page: 100,
			});

			const changesRequestedReview = reviews
				.reverse()
				.find((review) => review.state === "CHANGES_REQUESTED");

			let lastTriggerTime = 0;
			const triggerReasons: string[] = [];

			if (labelEvent?.created_at) {
				const labelTime = new Date(labelEvent.created_at).getTime();
				if (labelTime > lastTriggerTime) {
					lastTriggerTime = labelTime;
				}
				triggerReasons.push(`🏷️ 标签 "${LABEL_NAME}"`);
			}

			if (changesRequestedReview?.submitted_at) {
				const reviewTime = new Date(
					changesRequestedReview.submitted_at,
				).getTime();
				if (reviewTime > lastTriggerTime) {
					lastTriggerTime = reviewTime;
				}
				triggerReasons.push("📝 Review 请求更改");
			}

			if (lastTriggerTime === 0) {
				console.log(`    ⚪ 无待更新标签或变更请求，跳过`);
				continue;
			}

			const daysSinceTrigger = (now - lastTriggerTime) / msPerDay;

			console.log(`    🧐 触发原因: ${triggerReasons.join(" & ")}`);
			console.log(`    ⏳ 触发状态距今: ${daysSinceTrigger.toFixed(1)} 天`);

			if (daysSinceTrigger > DAYS_THRESHOLD) {
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
						body: `你好，由于此 PR 需要更新，但超过 ${DAYS_THRESHOLD} 天未更新，我们已将其关闭。如需继续贡献歌词，请重新打开一个新的 PR。`,
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
			} else {
				console.log(`    ⏭️ 触发时间未超过 ${DAYS_THRESHOLD} 天，跳过`);
			}
		} catch (error) {
			console.error(`💥 处理 PR #${pr.number} 时出错`, error);
		}
	}
}

run();
