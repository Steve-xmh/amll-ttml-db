import { exec } from "child_process";

export const githubToken = process.env.GITHUB_TOKEN;
export const [REPO_OWNER, REPO_NAME] = process.env.GITHUB_REPOSITORY?.split("/") ?? [
	"Steve-xmh",
	"amll-ttml-db",
];

export const HAS_CHECKED_MARK = "<!-- AMLL-DB-BOT-CHECKED -->";

export function parseBody(body) {
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

export function getMetadata(ttml, key) {
	const metadata = ttml.metadata.filter((v) => v.key === key).map((v) => v.value);
	const result = [];
	metadata.forEach(meta => {
		result.push(...meta);
	});
	return result;
}

/**
 * @returns {Promise<boolean>}
 */
export function isGitWorktreeClean() {
    return new Promise((resolve, reject) => {
        exec("git status --porcelain", (err, stdout, stderr) => {
            if (err) {
                reject(err);
            } else {
                if (stdout.length > 0) {
                    resolve(false);
                } else {
                    resolve(true);
                }
            }
        }
        );
    });
}

/**
 * @param {string} file
 * @returns {Promise<void>}
 */
export function addFileToGit(file) {
    return new Promise((resolve, reject) => {
        exec(`git add ${file}`, (err, stdout, stderr) => {
            if (err) {
                reject(err);
            } else {
                resolve();
            }
        }
        );
    });
}

/**
 * @param {string} branch
 * @returns {Promise<void>}
 */
export function checkoutBranch(branch) {
    return new Promise((resolve, reject) => {
        exec(`git checkout ${branch}`, (err, stdout, stderr) => {
            if (err) {
                reject(err);
            } else {
                resolve();
            }
        }
        );
    });
}

/**
 * @param {string} branch
 * @returns {Promise<void>}
 */
export function createBranch(branch) {
    return new Promise((resolve, reject) => {
        exec(`git checkout --force -b ${branch}`, (err, stdout, stderr) => {
            if (err) {
                reject(err);
            } else {
                resolve();
            }
        }
        );
    });
}

/**
 * @param {string} branch
 * @returns {Promise<void>}
 */
export function deleteBranch(branch) {
    return new Promise((resolve, reject) => {
        exec(`git branch -D ${branch}`, (err, stdout, stderr) => {
            if (err) {
                reject(err);
            } else {
                resolve();
            }
        }
        );
    });
}

/**
 * @param {string} message
 * @returns {Promise<void>}
 */
export function commit(message) {
    return new Promise((resolve, reject) => {
        exec(`git commit -m "${message}"`, (err, stdout, stderr) => {
            if (err) {
                reject(err);
            } else {
                resolve();
            }
        }
        );
    });
}

/**
 * @param {string} branch
 * @returns {Promise<void>}
 */
export function push(branch) {
    return new Promise((resolve, reject) => {
        exec(`git push --set-upstream origin ${branch}`, (err, stdout, stderr) => {
            if (err) {
                reject(err);
            } else {
                resolve();
            }
        }
        );
    });
}

