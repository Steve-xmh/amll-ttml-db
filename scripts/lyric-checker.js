/**
 *
 * @param {any[]} lyric
 * @returns {string[]}
 */
export function checkLyric(lyric) {
	const errors = [];
	const indexedLyric = lyric.map((line, id) => ({ ...line, id }));

	if (indexedLyric.length === 0) {
		errors.push("歌词内容为空");
	}
	for (const line of indexedLyric) {
		if (line.originalLyric.trim().length > 0) {
			const moreSpace = /\s\s+/
			if (moreSpace.test(line.originalLyric)) {
				errors.push(
					`第 ${line.id + 1} 行歌词内容中有多余的空格`,
				);
			}
			if (line.dynamicLyric) {
				line.dynamicLyric.forEach((word, wordIndex) => {
					if (word.word.trim().length > 0) {
						if (word.time < 0) {
							errors.push(
								`第 ${line.id + 1} 行歌词的第 ${wordIndex + 1} 个单词 "${
									word.word
								}" 开始时间有误 (${word.time})`,
							);
						}
						if (word.duration < 0) {
							errors.push(
								`第 ${line.id + 1} 行歌词的第 ${wordIndex + 1} 个单词 "${
									word.word
								}" 时长有误 (${word.duration})`,
							);
						}
					}
				});
			} else {
				if (line.beginTime < 0) {
					errors.push(
						`第 ${line.id + 1} 行歌词 开始时间有误 (${line.beginTime})`,
					);
				}
				if (line.duration <= 0) {
					errors.push(`第 ${line.id + 1} 行歌词 时长有误 (${line.duration})`);
				}
			}
		} else {
			errors.push(`第 ${line.id + 1} 行歌词内容为空`);
		}
	}

	return errors;
}
