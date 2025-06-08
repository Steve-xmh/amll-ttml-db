import { exec, spawn } from "child_process";

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
        // 执行 git commit，并获取其 stdin
        const gitCommit = spawn('git', ['commit', '-F', '-']);

        let stderr = '';
        // 捕获错误输出
        gitCommit.stderr.on('data', (data) => {
            stderr += data.toString();
        });

        // 监听进程退出事件
        gitCommit.on('close', (code) => {
            if (code === 0) {
                resolve();
            } else {
                console.error("Git commit 命令失败, 错误: ", stderr);
                reject(new Error(`Git commit 进程退出, 代码: ${code}`));
            }
        });
        
        // 监听执行错误事件
        gitCommit.on('error', (err) => {
            reject(err);
        });

        // 将 message 写入 git commit 进程的 stdin
        gitCommit.stdin.write(message);
        gitCommit.stdin.end();
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


/**
 * 规范化所有空白字符
 * @param {string} text 原始文本。
 * @param {string} context 上下文，用于日志记录.
 * @param {string[]} logs 日志收集器。
 * @returns {{normalized: string, changed: boolean}} 返回一个包含规范化后文本和变更状态的对象。
 */
export function normalizeString(text, context, logs) {
  if (typeof text !== 'string' || !text) {
    return { normalized: '', changed: false };
  }
  // 按任意空白分割，用单个空格连接，再移除首尾空格。
  const normalized = text.split(/\s+/).join(' ').trim();
  const changed = normalized !== text;
  if (changed) {
    const logMessage = `- **${context}**: 格式不规范，已自动清理。\n  - **原始**: \`${text}\`\n  - **修正**: \`${normalized}\``;
    if (!logs.includes(logMessage)) {
        logs.push(logMessage);
    }
  }
  return { normalized, changed };
}


/**
 * 规范化单行歌词
 * @param {object} line 包含 words 数组的歌词行对象。
 * @param {number} lineIndex 当前行的索引，用于日志。
 * @param {string[]} logs 日志收集器。
 * @returns {{newLine: object, changed: boolean}} 返回包含处理后行对象和变更状态的对象。
 */
export function normalizeLyricLine(line, lineIndex, logs) {
    if (!line || !line.words || line.words.length === 0) {
        return { newLine: line, changed: false };
    }

    const originalWordsString = JSON.stringify(line.words);
    const finalWords = [];
    let hasChanges = false;
    // 计数前面有多少个空格
    let pendingSpaces = 0;

    /**
     * 日志记录的辅助函数
     * @param {string} message - 日志内容
     */
    const logChange = (message) => {
        const logMessage = `- **第 ${lineIndex + 1} 行主歌词**: ${message}`;
        if (!logs.includes(logMessage)) {
            logs.push(logMessage);
        }
        hasChanges = true;
    };

    // 辅助函数，用于处理累积的空格
    const processPendingSpaces = () => {
        if (pendingSpaces === 0) {
            return;
        }

        if (finalWords.length > 0) {
            // 是词间空格
            finalWords.push({ word: ' ', startTime: 0, endTime: 0 });
            if (pendingSpaces > 1) {
                logChange(`合并了 ${pendingSpaces} 处不规范的词间空格。`);
            }
        } else {
            // 是行首空格
            logChange(`移除了 ${pendingSpaces} 处不规范的首尾空格。`);
        }
        pendingSpaces = 0;
    };

    for (const wordObj of line.words) {
        if (!wordObj || typeof wordObj.word !== 'string') {
            logChange(`发现并跳过了一个无效的歌词音节。`);
            continue;
        }

        // 将分解为前导空格、核心文本和尾随空格
        const match = wordObj.word.match(/^(\s*)([\s\S]*?)(\s*)$/);
        const leadingSpace = match[1];
        const coreText = match[2];
        const trailingSpace = match[3];

        // 整个音节都由空格组成
        if (coreText === '') {
            // 只要有内容，就视为一个空格来源
            if (wordObj.word.length > 0) {
                pendingSpaces++;
                logChange(`发现了一个完全由空格组成的音节 \`${wordObj.word}\``);
            }
            continue;
        }

        // 遇到了一个包含实际文本的音节
        
        // 前导空格
        if (leadingSpace) {
            pendingSpaces++;
            logChange(`提取了音节 \`${wordObj.word}\` 的前导空格。`);
        }
        
        // 更新所有在这之前的空格（上个词的尾随空格和这个词的前导空格）
        processPendingSpaces();

        // 规范化
        const normalizedCoreText = coreText.split(/\s+/).join(' ');
        if (coreText !== normalizedCoreText) {
            logChange(`规范化了音节 \`${coreText}\` 内部的空格。`);
        }
        finalWords.push({ ...wordObj, word: normalizedCoreText });

        if (trailingSpace) {
            pendingSpaces++;
            logChange(`提取了音节 \`${wordObj.word}\` 的尾随空格。`);
        }
    }

    // 处理所有遗留的句尾空格
    if (pendingSpaces > 0) {
        logChange(`移除了 ${pendingSpaces} 处不规范的首尾空格。`);
    }

    const changed = hasChanges || (JSON.stringify(finalWords) !== originalWordsString);
    const newLine = { ...line, words: finalWords };
    return { newLine, changed };
}
