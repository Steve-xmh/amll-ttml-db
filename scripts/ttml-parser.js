// @ts-nocheck
/**
 * @fileoverview
 * 解析 TTML 歌词文档到歌词数组的解析器
 * 用于解析从 Apple Music 来的歌词文件，且扩展并支持翻译和音译文本。
 * @see https://www.w3.org/TR/2018/REC-ttml1-20181108/
 */
import { JSDOM } from "jsdom";

const timeRegexp =
	/^(((?<hour>[0-9]+):)?(?<min>[0-9]+):)?(?<sec>[0-9]+([.:]([0-9]+))?)/;
function parseTimespan(timeSpan) {
	const matches = timeRegexp.exec(timeSpan);
	if (matches) {
		const hour = Number(matches.groups?.hour || "0");
		const min = Number(matches.groups?.min || "0");
		const sec = Number(matches.groups?.sec.replace(/:/, ".") || "0");
		return Math.floor((hour * 3600 + min * 60 + sec) * 1000);
	} else {
		throw new TypeError(`时间戳字符串解析失败：${timeSpan}`);
	}
}

function parseLyricInner(ttmlDoc) {
	let mainAgentId = "v1";

	const metadata = [];
	for (const meta of ttmlDoc.querySelectorAll("amll\\:meta")) {
		if (meta.tagName.toLowerCase() === "amll:meta") {
			const key = meta.getAttribute("key");
			if (key) {
				const value = meta.getAttribute("value");
				if (value) {
					const existing = metadata.find((m) => m.key === key);
					if (existing) {
						existing.value.push(value);
					} else {
						metadata.push({
							key,
							value: [value],
						});
					}
				}
			}
		}
	}

	for (const agent of ttmlDoc.querySelectorAll("ttm\\:agent")) {
		if (agent.getAttribute("type") === "person") {
			const id = agent.getAttribute("xml:id");
			if (id) {
				mainAgentId = id;
        break;
			}
		}
	}

	const lyricLines = [];

	function parseParseLine(lineEl, isBG = false) {
		const line = {
			words: [],
			translatedLyric: "",
			romanLyric: "",
			isBG,
			isDuet:
				!!lineEl.getAttribute("ttm:agent") &&
				lineEl.getAttribute("ttm:agent") !== mainAgentId,
			startTime: 0,
			endTime: 0,
		};
		let haveBg = false;

		const startTime = lineEl.getAttribute("begin");
		const endTime = lineEl.getAttribute("end");
		if (startTime && endTime) {
			line.startTime = parseTimespan(startTime);
			line.endTime = parseTimespan(endTime);
		} else {
			line.startTime = line.words.reduce(
				(pv, cv) => Math.min(pv, cv.startTime),
				Infinity,
			);
			line.endTime = line.words.reduce((pv, cv) => Math.max(pv, cv.endTime), 0);
		}

		for (const wordNode of lineEl.childNodes) {
			if (wordNode.nodeType === ttmlDoc.TEXT_NODE) {
				const word = wordNode.textContent ?? "";
				line.words.push({
					word: word,
					startTime: word.trim().length > 0 ? line.startTime : 0,
					endTime: word.trim().length > 0 ? line.endTime : 0,
				});
			} else if (wordNode.nodeType === ttmlDoc.ELEMENT_NODE) {
				const wordEl = wordNode;
				const role = wordEl.getAttribute("ttm:role");

				if (wordEl.nodeName.toLowerCase() === "span" && role) {
					if (role === "x-bg") {
						parseParseLine(wordEl, true);
						haveBg = true;
					} else if (role === "x-translation") {
						line.translatedLyric = wordEl.textContent ?? "";
					} else if (role === "x-roman") {
						line.romanLyric = wordEl.textContent ?? "";
					}
				} else if (wordEl.hasAttribute("begin") && wordEl.hasAttribute("end")) {
					const word = {
						word: wordNode.textContent ?? "",
						startTime: parseTimespan(wordEl.getAttribute("begin") ?? ""),
						endTime: parseTimespan(wordEl.getAttribute("end") ?? ""),
					};
					const emptyBeat = wordEl.getAttribute("amll:empty-beat");
					if (emptyBeat) {
						word.emptyBeat = Number(emptyBeat);
					}
					line.words.push(word);
				}
			}
		}

		if (line.isBG) {
			const firstWord = line.words[0];
			if (firstWord?.word?.startsWith("(")) {
				firstWord.word = firstWord.word.substring(1);
				if (firstWord.word.length === 0) {
					line.words.shift();
				}
			}

			const lastWord = line.words[line.words.length - 1];
			if (lastWord?.word?.endsWith(")")) {
				lastWord.word = lastWord.word.substring(0, lastWord.word.length - 1);
				if (lastWord.word.length === 0) {
					line.words.pop();
				}
			}
		}


		if (haveBg) {
			const bgLine = lyricLines.pop();
			lyricLines.push(line);
			if (bgLine) lyricLines.push(bgLine);
		} else {
			lyricLines.push(line);
		}
	}

	for (const lineEl of ttmlDoc.querySelectorAll("body p[begin][end]")) {
		parseParseLine(lineEl);
	}

	return {
		metadata,
		lyricLines: lyricLines,
	};
}


export function parseLyric(ttmlText) {
	const win = new JSDOM(ttmlText).window;
	const Node = win.Node;
	const ttmlDoc = win.document;

	return parseLyricInner(ttmlDoc);
}
