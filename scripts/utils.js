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

    /**
     * 日志记录的辅助函数
     * @param {string} message - 日志内容
     * @param {boolean} [isWarning=false] - 是否为警告日志
     */
    const logChange = (message, isWarning = false) => {
        const logMessage = `- **第 ${lineIndex + 1} 行主歌词**: ${message}`;
        if (!logs.includes(logMessage)) {
            logs.push(logMessage);
        }
        // 只有修复了才算作 hasChanges，纯警告不算
        if (!isWarning) {
            hasChanges = true;
        }
    };

    let pendingSpaceObjects = [];

    // 辅助函数，用于处理累积的空格
    const processPendingSpaces = (isTrailing = false) => {
        if (pendingSpaceObjects.length === 0) return;

        if (isTrailing || finalWords.length === 0) {
            // 如果是句首或句尾空格，则记录为“移除”
            logChange(`移除了 ${pendingSpaceObjects.length} 个不规范的首尾空格。`);
        } else {
            // 如果是词间空格，则记录为“合并”
            logChange(`合并了 ${pendingSpaceObjects.length} 个不规范的词间空格。`);
            finalWords.push({ word: ' ', startTime: 0, endTime: 0 });
        }
        pendingSpaceObjects = [];
    };

    for (const wordObj of line.words) {
        if (!wordObj || typeof wordObj.word !== 'string') {
            logChange(`发现并跳过了一个无效的歌词音节。`, true);
            continue;
        }

        const isSpace = wordObj.startTime === 0 && wordObj.endTime === 0;

        if (isSpace) {
             if (wordObj.word.trim() === '') {
                // 无时间戳的空格
                pendingSpaceObjects.push(wordObj);
             } else {
                // 无时间戳但有内容
                processPendingSpaces(); // 处理掉前面的空格
                logChange(`保留了一个无时间戳的音节: \`${wordObj.word}\`。请检查其格式。`, true);
                finalWords.push(wordObj);
             }
        } else {
            processPendingSpaces();

            const originalText = wordObj.word;
            const normalizedText = originalText.trim().split(/\s+/).join(' ');
            let wordToPush = wordObj;

            if (originalText !== normalizedText) {
                if (normalizedText === '') {
                     wordToPush = { ...wordObj, word: ' ' };
                } else {
                    logChange(`规范化了音节 \`${originalText}\` 内部的空格。`);
                    wordToPush = { ...wordObj, word: normalizedText };
                }
            }
            finalWords.push(wordToPush);
        }
    }

    // 处理所有遗留的句尾空格
    processPendingSpaces(true);

    const changed = hasChanges || (JSON.stringify(finalWords) !== originalWordsString);
    const newLine = { ...line, words: finalWords };
    return { newLine, changed };
}
