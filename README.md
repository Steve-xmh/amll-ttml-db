# AMLL TTML Database

> [!Important]
>
> This lyric database is for **Chinese language only**!
> If you wish to upload your lyrics, please keep lyric translation line blank and only include original lyric line and transliteration line. Thank you!

作者特供给 [AMLL](https://github.com/Steve-xmh/applemusic-like-lyrics) 的 TTML 逐词歌词库，也欢迎大家前来建设本仓库！

如果需要制作逐词歌词，可以使用 [AMLL TTML Tool](https://github.com/Steve-xmh/amll-ttml-tool) 哦！

为了避免非修正歌词情况下的歌词撞车问题，请先使用 [SearchInAMLLDB](https://steamfinder.github.io/search-in-amlldb/) 来检索你的歌曲是否拥有 TTML 歌词噢！
（感谢 [@SteamFinder](https://github.com/SteamFinder) 建立的检索站！）

## 歌词要求

歌词审核细则可以参考[ 这个文件 ](./instruction.md)，以下内容为简要基本要求概述。

### 歌词内容要求

#### 硬性要求

- 不要在歌词主体里包含除歌词内容以外的信息（例如 作词、作曲、歌词制作作者 这类信息）
  - 注：如确有需要，请通过 AMLL TTML Tool 元数据功能存储此类信息
- 不要留空行，请善用结束时间来让歌词播放器自动生成间奏区域
  - 注：Github 机器人会自动检查该部分
- 单词时序不能有误（例如开始时间比结束时间晚的）
  - 注：Github 机器人会自动检查该部分
- 如果是英文歌曲，确保单词间隔不会超过一个空格
  - 注：Github 机器人会自动检查该部分
- 如果是对已有歌词的修正，请在补充说明处写明修改原因
- 涉及政治敏感、有违人道主义的歌曲曲目不得提供歌词翻译（音译不限）（如果为国语歌曲则不予提交歌词）
  - 注：对于可能有 NSFW 内容的歌词内容翻译，不反对表达原意但请尽量把握尺度，做到达意后最好点到为止，否则可能视情况会推迟审核甚至驳回

#### 优先审核要求

- 尽量是逐词歌词，会优先审核，且确保打轴时序差别在 ±100 毫秒以内
  - 注：如果条件所限可以提交逐行歌词，而后可以通过修正提交来提交逐词歌词
- 尽量利用好 TTML 歌词特性，例如背景人声歌词和对唱人声歌词，会优先审核（但不建议矫枉过正，歌曲不存在背景或对唱的情况下不要强加效果）
- 尽可能提供翻译和音译（如果有）
  - 注：对于不使用 AMLL TTML Tool 的歌词制作者或歌词编辑器开发者，你可以在每行的 `p` 元素中加入 `span[ttm:role=x-translation]` 作为翻译文本或者 `span[ttm:role=x-roman]` 作为音译文本。具体可以参考下方的歌词范例。

#### 常见驳回原因

为了提高歌词库的歌词统一性和综合质量，除了上述情况以外，部分偏主观的情况也容易导致你的歌词作品被驳回，还请多多留意，具体有可能是以下原因：

- 矫枉过正的类型，有可能有以下情况：
  - 即不适用于部分效果时却强行加入效果，这会对歌词的观感产生多余影响
  - 为了触发某些歌词效果而刻意错误打轴，这会导致歌词可能因为所展示的载体更新迭代导致显示误差的增加
- 使用错误的类型，有可能有以下情况：
  - 不能正确区分当前歌手是处于辅助主唱还是独自对唱演唱的状态，以至于错误使用了背景歌词和对唱歌词
  - 除非歌词本身不为歌曲考虑使用对唱歌词，否则应该要根据当前演唱者主次关系设置正确的对唱歌词，在没有确切动机的情况下不应将同一演唱者的同一演唱情况设置出两种不同的歌词状态

如果你的作品被驳回，且你认为审核员所主观判断的情况并无大碍，可以重复提交并请求其他审核员一同检查，或是留下备注信息以便审核员理解你的意图。

### 歌词提交要求

请提供和 Apple Music 所需 TTML 格式歌词要求一致的歌词文件，如果可以请尽量制作逐词歌词，并命名成 `[提交UNIX时间戳]-[提交者GithubID]-[8位随机ID].ttml` 后存放于 `./raw-lyrics` 文件夹后发送 Pull Request 提交歌词即可。

或者根据议题模板提交模板，Github Action 会自动检查歌词文件格式并为你创建合并请求。

## 歌词文件范例

- [Taylor Swift,Brendon Urie - ME! (feat. Brendon Urie of Panic! At The Disco) （含对唱/背景人声歌词）](./ncm-lyrics/1361348080.ttml)
- [Jake Miller, HOYO-MiX - WHITE NIGHT (不眠之夜）（含对唱/背景人声歌词）](./ncm-lyrics/2122308128.ttml)
- [ナユタン星人, 初音ミク - 太陽系デスコ（含对唱/背景人声歌词）](./ncm-lyrics/459717345.ttml)

（歌词文件非常标准的也可以在 PR 时给本列表增加项目）

## 使用歌词数据库

> [!TIP]
> 虽然这并非强制，但我们希望你在使用本歌词数据库时，能够在你的项目中加入一个指向本仓库或者衍生项目的链接或说明，或是展示每个歌词文件中的歌词作者信息（均已在元数据中可以读取），以便更多人能够发现这个数据库，一同建设本歌词数据库。

如果你想要接入本歌词数据库，可以通过各类以平台类型区分的文件夹，用您对应平台的音乐 ID 来获取歌词文件。

现阶段支持以下平台的歌词索引：

- [Netease Cloud Music - 网易云音乐](./ncm-lyrics) 
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

同时，在每个平台文件夹下，还有一个存有基本信息的 `index.jsonl` 逐行存储了该平台下所属的所有歌词基本信息，以原始歌词文件顺序排列，也列出了所有历史歌词信息。

## 共享协议

本仓库的外来数据部分遵循原数据提供方的共享协议共享，提交者自主编写的部分使用 CC0 1.0 共享协议共享。

## 鸣谢

感谢所有为建设本仓库提供歌词的贡献者们！

[![贡献者头像画廊，点击可查阅](https://amll-ttml-db.stevexmh.net/contributors.png)](./CONTRIBUTORS.md)
