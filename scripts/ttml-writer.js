/**
 * @fileoverview
 * 用于将内部歌词数组对象导出成 TTML 格式的模块
 * 但是可能会有信息会丢失
 */
import { JSDOM } from "jsdom";
import prettier from "prettier";

function msToTimestamp(timeMS) {
	if (!Number.isSafeInteger(timeMS) || timeMS < 0) {
		return "00:00.000";
	}
	if (timeMS === Infinity) {
		return "99:99.999";
	}
	timeMS = timeMS / 1000;
	const secs = timeMS % 60;
	timeMS = (timeMS - secs) / 60;
	const mins = timeMS % 60;
	const hrs = (timeMS - mins) / 60;

	const h = hrs.toString().padStart(2, "0");
	const m = mins.toString().padStart(2, "0");
	const s = secs.toFixed(3).padStart(6, "0");

	if (hrs > 0) {
		return `${h}:${m}:${s}`;
	} else {
		return `${m}:${s}`;
	}
}

function exportTTMLTextInner(
	doc,
	ttmlLyric,
) {
	const params = [];
	const lyric = ttmlLyric.lyricLines;

	let tmp = [];
	for (const line of lyric) {
		if (line.words.length === 0 && tmp.length > 0) {
			params.push(tmp);
			tmp = [];
		} else {
			tmp.push(line);
		}
	}

	if (tmp.length > 0) {
		params.push(tmp);
	}

	function createWordElement(word) {
		const span = doc.createElement("span");
		span.setAttribute("begin", msToTimestamp(word.startTime));
		span.setAttribute("end", msToTimestamp(word.endTime));
		if (word.emptyBeat) {
			span.setAttribute("amll:empty-beat", `${word.emptyBeat}`);
		}
		span.appendChild(doc.createTextNode(word.word));
		return span;
	}

	const ttRoot = doc.querySelector("tt");

	ttRoot.setAttribute("xmlns", "http://www.w3.org/ns/ttml");
	ttRoot.setAttribute("xmlns:ttm", "http://www.w3.org/ns/ttml#metadata");
	ttRoot.setAttribute("xmlns:amll", "http://www.example.com/ns/amll");
	ttRoot.setAttribute(
		"xmlns:itunes",
		"http://music.apple.com/lyric-ttml-internal",
	);

	const head = doc.querySelector("head");

	ttRoot.appendChild(head);

	const body = doc.querySelector("body");
	const hasOtherPerson = !!lyric.find((v) => v.isDuet);

	const metadataEl = doc.createElement("metadata");
	const mainPersonAgent = doc.createElement("ttm:agent");
	mainPersonAgent.setAttribute("type", "person");
	mainPersonAgent.setAttribute("xml:id", "v1");

	metadataEl.appendChild(mainPersonAgent);

	if (hasOtherPerson) {
		const otherPersonAgent = doc.createElement("ttm:agent");
		otherPersonAgent.setAttribute("type", "other");
		otherPersonAgent.setAttribute("xml:id", "v2");

		metadataEl.appendChild(otherPersonAgent);
	}

	for (const metadata of ttmlLyric.metadata) {
		for (const value of metadata.value) {
			const metaEl = doc.createElement("amll:meta");
			metaEl.setAttribute("key", metadata.key);
			metaEl.setAttribute("value", value);
			metadataEl.appendChild(metaEl);
		}
	}

	head.appendChild(metadataEl);

	let i = 0;

	const guessDuration = lyric[lyric.length - 1]?.endTime ?? 0;
	body.setAttribute("dur", msToTimestamp(guessDuration));

	for (const param of params) {
		const paramDiv = doc.createElement("div");
		const beginTime = param[0]?.startTime ?? 0;
		const endTime = param[param.length - 1]?.endTime ?? 0;

		paramDiv.setAttribute("begin", msToTimestamp(beginTime));
		paramDiv.setAttribute("end", msToTimestamp(endTime));

		for (let lineIndex = 0; lineIndex < param.length; lineIndex++) {
			const line = param[lineIndex];
			const lineP = doc.createElement("p");
			const beginTime = line.startTime ?? 0;
			const endTime = line.endTime;

			lineP.setAttribute("begin", msToTimestamp(beginTime));
			lineP.setAttribute("end", msToTimestamp(endTime));

			lineP.setAttribute("ttm:agent", line.isDuet ? "v2" : "v1");
			lineP.setAttribute("itunes:key", `L${++i}`);

			if (line.words.length === 1) {
                lineP.appendChild(doc.createTextNode(line.words[0].word));
            } else {
                for (const word of line.words) {
                    if (word.word === ' ') {
                        lineP.appendChild(doc.createTextNode(' '));
                    } else {
                        const span = createWordElement(word);
                        lineP.appendChild(span);
                    }
                }
            }

			const nextLine = param[lineIndex + 1];
			if (nextLine && nextLine.isBG) {
				lineIndex++;
				const bgLine = nextLine;
				const bgLineSpan = doc.createElement("span");
				bgLineSpan.setAttribute("ttm:role", "x-bg");

				if (bgLine.words.length > 1) {
					let beginTime = Infinity;
					let endTime = 0;
					for (let wordIndex = 0; wordIndex < bgLine.words.length; wordIndex++) {
						const word = bgLine.words[wordIndex];
						if (word.word.trim().length === 0) {
							bgLineSpan.appendChild(doc.createTextNode(word.word));
						} else {
							const span = createWordElement(word);
							if (wordIndex === 0) {
								span.prepend(doc.createTextNode("("));
							} else if (wordIndex === bgLine.words.length - 1) {
								span.appendChild(doc.createTextNode(")"));
							}
							bgLineSpan.appendChild(span);
							beginTime = Math.min(beginTime, word.startTime);
							endTime = Math.max(endTime, word.endTime);
						}
					}
					bgLineSpan.setAttribute("begin", msToTimestamp(beginTime));
					bgLineSpan.setAttribute("end", msToTimestamp(endTime));
				} else if (bgLine.words.length === 1) {
					const word = bgLine.words[0];
					bgLineSpan.appendChild(doc.createTextNode(`(${word.word})`));
					bgLineSpan.setAttribute("begin", msToTimestamp(word.startTime));
					bgLineSpan.setAttribute("end", msToTimestamp(word.endTime));
				}

				if (bgLine.translatedLyric) {
					const span = doc.createElement("span");
					span.setAttribute("ttm:role", "x-translation");
					span.setAttribute("xml:lang", "zh-CN");
					span.appendChild(doc.createTextNode(bgLine.translatedLyric));
					bgLineSpan.appendChild(span);
				}

				if (bgLine.romanLyric) {
					const span = doc.createElement("span");
					span.setAttribute("ttm:role", "x-roman");
					span.appendChild(doc.createTextNode(bgLine.romanLyric));
					bgLineSpan.appendChild(span);
				}

				lineP.appendChild(bgLineSpan);
			}

			if (line.translatedLyric) {
				const span = doc.createElement("span");
				span.setAttribute("ttm:role", "x-translation");
				span.setAttribute("xml:lang", "zh-CN");
				span.appendChild(doc.createTextNode(line.translatedLyric));
				lineP.appendChild(span);
			}

			if (line.romanLyric) {
				const span = doc.createElement("span");
				span.setAttribute("ttm:role", "x-roman");
				span.appendChild(doc.createTextNode(line.romanLyric));
				lineP.appendChild(span);
			}

			paramDiv.appendChild(lineP);
		}

		body.appendChild(paramDiv);
	}

	ttRoot.appendChild(body);
}


export async function exportTTMLText(lyric, pretty = false) {
	const jsdom = new JSDOM(
		`<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" xmlns:itunes="http://music.apple.com/lyric-ttml-internal"><head></head><body></body></tt>`,
		{
			contentType: "application/xml",
		},
	);
	const doc = jsdom.window.document;
	exportTTMLTextInner(doc, lyric);
	if (pretty) {
		return await prettier.format(jsdom.serialize(), { parser: "html" });
	} else {
		return jsdom.serialize();
	}
}
