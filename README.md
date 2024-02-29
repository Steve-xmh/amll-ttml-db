# AMLL TTML Database

作者特供给 [AMLL](https://github.com/Steve-xmh/applemusic-like-lyrics) 的 TTML 逐词歌词库，也欢迎大家前来建设本仓库！

如果需要制作逐词歌词，可以使用 [AMLL TTML Tools](https://github.com/Steve-xmh/amll-ttml-tool) 哦！

为了避免非修正歌词情况下的歌词撞车问题，请先使用 [SearchInAMLLDB](https://steamfinder.github.io/search-in-amlldb/) 来检索你的歌曲是否拥有 TTML 歌词噢！
（感谢 [@SteamFinder](https://github.com/SteamFinder) 建立的检索站！）

## 歌词要求

### 歌词内容要求

#### 硬性要求

- 不要在歌词主体里包含除歌词内容以外的信息（例如 作词、作曲 这类信息）
- 不要留空行，请善用结束时间来让歌词播放器自动生成间奏区域
- 单词时序不能有误（例如开始时间比结束时间晚的）
- 如果是英文歌曲，确保单词间隔不会超过一个空格
- 如果是对已有歌词的修正，请在补充说明处写明修改原因

#### 优先审核要求

- 尽量是逐词歌词，会优先审核
- 尽量利用好 TTML 歌词特性，例如背景人声歌词和对唱人声歌词，会优先审核（但不建议矫枉过正，歌曲不存在背景或对唱的情况下不要强加效果）
- 尽可能提供翻译和音译（如果有），你可以在每行 `p` 元素中加入 `span[ttm:role=x-translation]` 作为翻译文本或者 `span[ttm:role=x-roman]` 作为音译文本。具体可以参考下方的歌词范例。

### 歌词提交要求

请提供和 Apple Music 所需 TTML 格式歌词要求一致的歌词文件，如果可以请尽量制作逐词歌词，并命名成 `歌曲在网易云上的音乐ID.ttml` 后存放于 `./lyrics` 文件夹后发送 Pull Request 提交歌词即可。

或者根据议题模板提交模板，Github Action 会自动检查歌词文件格式并为你创建合并请求。

## 歌词文件范例

- [Taylor Swift,Brendon Urie - ME! (feat. Brendon Urie of Panic! At The Disco) （含对唱/背景人声歌词）](./ncm-lyrics/1361348080.ttml)
- [Jake Miller, HOYO-MiX - WHITE NIGHT (不眠之夜）（含对唱/背景人声歌词）](./ncm-lyrics/2122308128.ttml)

（歌词文件非常标准的也可以在 PR 时给本列表增加项目）

## 使用歌词数据库

> [!TIP]
> 虽然这并非强制，但我们希望你在使用本歌词数据库时，能够在你的项目中加入一个指向本仓库或者衍生项目的链接或说明，或是展示每个歌词文件中的歌词作者信息（均已在元数据中可以读取），以便更多人能够发现这个数据库，一同建设本歌词数据库。

如果你想要接入本歌词数据库，可以通过各类以平台类型区分的文件夹，用您对应平台的音乐ID来获取歌词文件。

现阶段支持以下平台的歌词索引：

- [Netease Cloud Music - 网易云音乐](./ncm-lyrics) （注：原 `lyrics` 文件夹依然保留且一并同步，但不再推荐使用该路径）
- [QQ Music - QQ 音乐](./qq-lyrics)
- [Apple Music](./apple-lyrics)
- [Spotify](./spotify-lyrics)

每个歌词 ID 均已自动生成不同格式的歌词文件，通过文件后缀名区分：
- `.ttml`: 原 TTML 歌词格式
- `.lrc`: LyRiC 歌词格式
- `.yrc`: 网易云音乐逐词歌词格式
- `.qrc`: QQ 音乐逐词歌词格式
- `.lys`: Lyricify Syllable 逐词歌词格式
- `.eslrc`: ESLrc 逐词歌词格式

如果需要检索从建立数据库开始至今所有的歌词文件，可以访问 [./raw-lyrics](./raw-lyrics) 文件夹，内部的文件以 `[提交UNIX时间戳]-[提交者GithubID]-[8位随机ID].ttml` 命名。

## 共享协议

本仓库的外来数据部分遵循原数据提供方的共享协议共享，提交者自主编写的部分使用 CC0 1.0 共享协议共享。
