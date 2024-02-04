import { mkdir, readFile, readdir, rm, copyFile } from "fs/promises";
import { resolve, join } from "path/posix";
import { parseLyric } from "./ttml-parser.js";
import { addFileToGit, commit, getMetadata, isGitWorktreeClean, push } from "./utils.js";

async function main() {
    console.log("正在重新构建文件夹");
    await rm("../lyrics", { force: true, recursive: true });
    await rm("../ncm-lyrics", { force: true, recursive: true });
    await rm("../spotify-lyrics", { force: true, recursive: true });
    await rm("../qq-lyrics", { force: true, recursive: true });
    await rm("../am-lyrics", { force: true, recursive: true });
    await mkdir("../lyrics", { recursive: true });
    await mkdir("../ncm-lyrics", { recursive: true });
    await mkdir("../spotify-lyrics", { recursive: true });
    await mkdir("../qq-lyrics", { recursive: true });
    await mkdir("../am-lyrics", { recursive: true });
    const rawLyricFiles = await readdir("../raw-lyrics");
    rawLyricFiles.sort((a, b) => {
        const aTime = parseInt(a.split("-")[0]);
        const bTime = parseInt(b.split("-")[0]);
        return aTime - bTime;
    });
    for (const file of rawLyricFiles) {
        console.log("正在处理", file);
        const filePath = resolve("../raw-lyrics", file);
        const lyricContent = await readFile(filePath, "utf-8");
        const lyric = parseLyric(lyricContent);

        for (const id of getMetadata(lyric, "ncmMusicId")) {
            await copyFile(filePath, join("../lyrics", `${id}.ttml`));
            await copyFile(filePath, join("../ncm-lyrics", `${id}.ttml`));
        }

        for (const id of getMetadata(lyric, "spotifyId")) {
            await copyFile(filePath, join("../spotify-lyrics", `${id}.ttml`));
        }

        for (const id of getMetadata(lyric, "qqMusicId")) {
            await copyFile(filePath, join("../qq-lyrics", `${id}.ttml`));
        }

        for (const id of getMetadata(lyric, "appleMusicId")) {
            await copyFile(filePath, join("../am-lyrics", `${id}.ttml`));
        }
    }
    console.log("文件夹重建完毕！");
    if (!(await isGitWorktreeClean())) {
        console.log("工作树已变更，正在提交更改");
        await addFileToGit("..");
        await commit(`于 ${new Date().toISOString()} 重新构建更新`);
        await push("main");
        console.log("更改提交完成！");
    }
}

await main();
