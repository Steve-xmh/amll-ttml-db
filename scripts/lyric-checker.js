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
		const originalLyric = line.words.map(v => v.word).join("").trim();
		if (originalLyric.length > 0) {
			const moreSpace = /\s\s+/;
			console.log(
				`正在检查第 ${line.id + 1} 行:`,
				JSON.stringify(originalLyric),
			);
			const moreSpaceRegResult = moreSpace.exec(originalLyric);
			if (moreSpaceRegResult) {
				errors.push(`第 ${line.id + 1} 行歌词内容中有多余的空格`);
			}
			line.words.forEach((word, wordIndex) => {
				if (word.word.trim().length > 0) {
					if (word.startTime < 0) {
						errors.push(
							`第 ${line.id + 1} 行歌词的第 ${wordIndex + 1} 个单词 "${
								word.word
							}" 开始时间有误 (${word.startTime})`,
						);
					}
					if (word.endTime < word.endTime) {
						errors.push(
							`第 ${line.id + 1} 行歌词的第 ${wordIndex + 1} 个单词 "${
								word.word
							}" 结束时间有误/小于开始时间 (${word.endTime})`,
						);
					}
				}
			});
			if (line.startTime < 0) {
				errors.push(
					`第 ${line.id + 1} 行歌词 开始时间有误 (${line.startTime})`,
				);
			}
			if (line.endTime < line.startTime) {
				errors.push(`第 ${line.id + 1} 行歌词 结束时间有误/小于开始时间 (${line.endTime})`);
			}
		} else {
			errors.push(`第 ${line.id + 1} 行歌词内容为空`);
		}
	}

	return errors;
}
