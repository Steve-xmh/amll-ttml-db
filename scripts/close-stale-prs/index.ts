import { Octokit } from "octokit";

const LABEL_NAME = "待更新";
const DAYS_THRESHOLD = 7;

const repoEnv = process.env.GITHUB_REPOSITORY || "";
const OWNER = process.env.OWNER || repoEnv.split("/")[0];
const REPO = process.env.REPO || repoEnv.split("/")[1];
const TOKEN = process.env.GITHUB_TOKEN;

const IS_DRY_RUN = process.env.DRY_RUN === "true";

if (!TOKEN) {
	console.error("缺少环境变量 GITHUB_TOKEN");
	process.exit(1);
}

if (!OWNER || !REPO) {
	console.error(
		"缺少仓库信息：请通过 OWNER/REPO 或 GITHUB_REPOSITORY 环境变量指定目标仓库",
	);
	process.exit(1);
}

const octokit = new Octokit({ auth: TOKEN });

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

const waitingPrs = prs.filter((pr) => {
	return pr.labels.some((label) => label.name === LABEL_NAME);
});

console.log(`🔍 找到 ${waitingPrs.length} 个带有 "${LABEL_NAME}" 标签的 PR。`);

for (const pr of waitingPrs) {
	console.log(`\n📋 检查 PR #${pr.number}: ${pr.title}`);

	try {
		const updatedAt = new Date(pr.updated_at).getTime();
		const daysSinceUpdate = (now - updatedAt) / msPerDay;

		if (daysSinceUpdate <= DAYS_THRESHOLD) {
			console.log(
				`    ✅ 距最后活跃仅 ${daysSinceUpdate.toFixed(1)} 天，未达到阈值，跳过。`,
			);
			continue;
		}

		console.log(
			`    🚫 满足关闭条件: 存在 "${LABEL_NAME}" 标签且 ${DAYS_THRESHOLD} 天无任何活跃`,
		);
		console.log(`    ⏳ 最后活跃距今: ${daysSinceUpdate.toFixed(1)} 天`);

		const branchName = pr.head.ref;
		const isSameRepo = pr.head.repo?.full_name === `${OWNER}/${REPO}`;
		const shouldDeleteBranch =
			isSameRepo && branchName.startsWith("auto-submit-issue");

		if (IS_DRY_RUN) {
			console.log(`    🔔 [DRY RUN] 满足关闭条件`);
			console.log(`        拟添加评论并关闭 PR #${pr.number}`);
			continue;
		}

		console.log(`    🚫 正在关闭此 PR...`);

		await octokit.rest.issues.createComment({
			owner: OWNER,
			repo: REPO,
			issue_number: pr.number,
			body: `你好，由于此 PR 需要修改，但超过 ${DAYS_THRESHOLD} 天没有任何更新，我们已将其关闭。欢迎重新贡献！`,
		});

		await octokit.rest.pulls.update({
			owner: OWNER,
			repo: REPO,
			pull_number: pr.number,
			state: "closed",
		});

		if (!shouldDeleteBranch) continue;

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
	} catch (error) {
		console.error(`💥 处理 PR #${pr.number} 时出错`, error);
	}
}
