<div align=center>

# **AMLL TTML DataBase**

这里是 Apple Music-like Lyrics 的 TTML 逐词歌词库，为 AMLL 更佳的歌词表现提供支持。

**—— AMLL 生态作品 ——**

[Apple Music-like Lyrics](https://github.com/Steve-xmh/applemusic-like-lyrics)
/
[AMLL TTML Tool 逐词歌词编辑器](https://github.com/Steve-xmh/amll-ttml-tool)

</div>

***

> [!Important]
>
> A note for non-Chinese contributors:
>
> This database is mainly for Chinese speakers. However, if you're translating lyrics into other languages, please specify it using the "xml:lang" attribute. If Chinese (or other language) version already exists, just keep all.
>
> Since AMLL series software currently does not support multilingual translations, users should get single-language-translated lyric by third-party tools (e.g. [ranhengzhang/ttml-trans-filter](https://github.com/ranhengzhang/ttml-trans-filter)) before using.
>
> Looking for more details? 👉[TTML Specification (Section 5.3)](https://github.com/Steve-xmh/amll-ttml-db/blob/main/instructions/ttml-specification-en.md#53-multi-language-and-background-support).
>
> ---
>
> *For demonstration purposes, the examples provided below are formatted. Please note that in an actual TTML file, the content is minified according to HTML standards. Keep this in mind when reading the examples and submitting files.*
>
> ---
>
> ###### Format 1:
>
> > **Example Code:**
> >
> > ```xml
> > <p begin="00:21.400" end="00:23.870" ttm:agent="v1" itunes:key="L1">
> >   <span begin="00:21.400" end="00:22.010">低</span>
> >   <span begin="00:22.200" end="00:23.010">等</span>
> >   <span begin="00:23.010" end="00:23.210">动</span>
> >   <span begin="00:23.210" end="00:23.870">物</span>
> >   <span ttm:role="x-translation" xml:lang="en-US">Lower beings</span>
> >   <span ttm:role="x-translation" xml:lang="ja-JP">劣等な生物たちよ</span>
> > </p>
> > ```
> >
> > **Example File:**
> >
> > [HOYOMiX/YMIR - 耀斑](https://github.com/Steve-xmh/amll-ttml-db/blob/main/raw-lyrics/1752080938784-68000793-8355bb14.ttml)
>
> ---
>
> ###### Format 2:
>
> > **Example Code:**
> >
> > ```xml
> > <iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal">
> >   <translations>
> >     <translation type="subtitle" xml:lang="zh-Hans">
> >       <text for="L1">黎明未至</text>
> >     </translation>
> >     <translation type="subtitle" xml:lang="el-GR">
> >       <text for="L1">Πριν απ&apos; την αυγή</text>
> >     </translation>
> >   </translations>
> > </iTunesMetadata>
> > ```
> >
> > **Example File:**
> >
> > [Darren Korb - Time Belongs to Us](https://github.com/Steve-xmh/amll-ttml-db/blob/main/raw-lyrics/1762708573944-68000793-qnjnaX11.ttml)
> >
> > ---
> >
> > *Although this format complies with the new standards of Apple Music, we strongly discourage using it because most programs utilizing amll-ttml-db cannot parse it. Furthermore, if you submit lyrics in this format via the Issue, the bot will automatically convert them to the first format.*

***

# 歌词提交流程

## 1. 检查是否重复提交

原则上，AMLL TTML DataBase 主要接受音源来自 [网易云音乐](https://music.163.com) 的歌词，以便其他用户使用，我们还接受音源来自 [Apple Music](https://music.apple.com)、 [QQ音乐](https://y.qq.com)、[Spotify](https://open.spotify.com) 的歌词，因此提交的 TTML 文件中应当至少包含以上四个平台之中一个平台的 ID。

### 在此仓库中检索是否有已提交歌词

请参考 [歌词元数据说明](https://github.com/Steve-xmh/amll-ttml-tool/wiki/%E6%AD%8C%E8%AF%8D%E5%85%83%E6%95%B0%E6%8D%AE) 获取您要提交歌词的歌曲 ID，在本仓库内搜索该 ID，如无任何文件，则该歌曲暂无 TTML 歌词，欢迎您的提交。

### 在 SearchInAMLLDB 中检索是否有已提交歌词

请访问 [SearchInAMLLDB](https://steamfinder.github.io/search-in-amlldb)，在最上方点击 <kbd>更新数据库</kbd> 按钮拉取数据，输入您要提交歌词的歌曲名称并点击 <kbd>查询</kbd> 按钮，如无任何结果，则该歌曲暂无 TTML 歌词，欢迎您的提交。

> 感谢 [@SteamFinder](https://github.com/SteamFinder) 建立的检索站！

### 检索是否有歌词制作占位议题

若您准备提交某首歌曲的歌词，创建 **「歌词制作占位」** 议题则代表您接手了该歌曲的歌词提交工作，以避免他人同时提交导致撞车。

> [!WARNING]
>
> 发布占位议题的投稿者，在发布议题后 7 日内提交的投稿将会被优先审核，无论这期间是否有其他人提交了相同的投稿。  
>
> 如果超出了这个时间范围，我们仍然按照投稿顺序进行审核。
>
> 时间范围以 **发布占位议题时系统显示的日期** 及 **PR被自动创建时系统显示的日期** 为准。
>
> 如果你发布了占位议题并提交了投稿，我们建议在备注中引用你的议题，以减少可能的疑议。

请访问 [此仓库/issue](https://github.com/Steve-xmh/amll-ttml-db/issues) ，搜索您要提交歌词的歌曲名称，如无 **Open 状态的、有「歌词制作占位」标签的** Issue，则该歌曲的歌词提交工作尚未被接手，欢迎您的提交。

您也可以访问 [AMLL TTML 歌词议题墙](https://amlldb.bikonoo.com/) 检索。

## 2. 制作歌词

### 歌词要求

歌词审核细则参考 [此文件](./instructions/instruction.md) ，以下内容为简要基本要求概述。

#### 硬性要求

- 请勿在歌词主体里包含除歌词内容以外的信息，例如 **作词**、**作曲**、**歌词制作者** 等信息；
  > 我们推荐通过 AMLL TTML Tool 元数据功能存储此类信息。
- 请勿留空行，善用 **结束时间** 以让歌词播放器自动生成间奏区域；
- 单词时序不能有误，例如 **开始时间** 比 **结束时间** 晚；
- 对于英文歌曲，单词之间的间隔不超过一个空格；
- 对于修正已有歌词，请在补充说明处写明修改原因以及原 Pr；
- 涉及政治敏感、有违人道主义的歌曲曲目不得提供歌词翻译（音译基本不限），如果为国语歌曲则不予提交歌词。
  > 对于可能有 NSFW 内容的歌词内容翻译，不反对表达原意，但请尽量把握尺度、点到为止，否则可能视情况推迟审核，甚至驳回歌词。
  > 
  > *NSFW：Not Safe For Work 的缩写，意思是某个网络内容不适合在工作场合浏览。它通常用于标记包含裸露、暴力或色情等不适合在工作环境中查看的内容。*

#### 优先审核要求

- 是逐词歌词，确保打轴时序差在 ±100 毫秒以内；
- 充分利用 TTML 歌词特性，例如有背景人声歌词和对唱歌词；
- 提供翻译和音译（如果有）。
  > 对于不使用 AMLL TTML Tool 的歌词制作者或歌词编辑器开发者，你可以在需要添加音译或翻译的行的 `p` 元素中加入 `<span ttm:role="x-translation" xml:lang="...">...</span>` 作为翻译文本或者 `<span ttm:role="x-roman">...</span>` 作为音译文本。

### 使用 AMLL TTML Tool 制作逐词歌词

我们推荐使用 [AMLL TTML Tool](https://amll-ttml-tool.stevexmh.net) 制作逐词歌词，此 README 将简要介绍 AMLL TTML Tool 的使用步骤。

您需要准备：

1. 可被 AMLL TTML Tool 读取的歌曲音频文件；
   > 受加密保护的格式不受支持。
2. 纯文本 或 其他格式的歌词文件。
   > 支持导入 LRC / ESLyric / YRC / QRC / Lyricify Syllable 格式的歌词文件。

然后：

1. 在左上角依次点击 <kbd>文件</kbd> <kbd>导入歌词</kbd> 并选择您导入歌词的格式，根据页面提示导入歌词；
2. 在左下角导入您的歌曲音频文件，调整播放倍速和音量；
3. 在左上角依次点击 <kbd>编辑</kbd> <kbd>编辑歌词元数据</kbd> ，参考 [歌词元数据说明](https://github.com/Steve-xmh/amll-ttml-db/blob/main/instructions/instruction.md#1-%E5%85%83%E6%95%B0%E6%8D%AE) 编辑歌词元数据；
4. 在 **编辑** 界面编辑您的歌词，如对歌词行分词 、更改歌词行属性、填写翻译和音译歌词等；
5. 在 **打轴** 界面制作逐词歌词，播放音频，善用以下按键开始打轴：
   | 按键         | 说明                                                         |
   | ------------ | ------------------------------------------------------------ |
   | <kbd>F</kbd> | 记录 **当前播放进度** 为当前单词的 **起始时间** 。           |
   | <kbd>G</kbd> | 记录 **当前播放进度** 为当前单词的 **结束时间** ，同时移动到下一个单词，并记录 **当前播放进度** 为该单词的 **起始时间** 。 |
   | <kbd>H</kbd> | 记录 **当前播放进度** 为当前单词的 **结束时间** ，然后移动到下一个单词。<br/>通常用于结束当前句子的单词，空出间奏区域，或是呈现歌手停顿式的演唱方式。 |
6. 完成打轴后，在 **预览** 界面预览您制作的逐词歌词；
7. 预览无误后，在左上角依次点击 <kbd>文件</kbd> <kbd>保存 TTML 歌词文件</kbd> 保存 TTML 歌词文件。

## 3. 提交歌词

我们推荐通过 [创建「提交/补正歌词」Issue](https://github.com/Steve-xmh/amll-ttml-db/issues/new?template=submit-lyric.yml) 的方式提交歌词，您可以在该页面查看详细的提交流程。

## 4. 等待审核

为了提高歌词库的歌词统一性和综合质量，您的歌词将由 AMLL TTML 歌词审核团进行人工审核，以确保您的歌词符合 [歌词审核细则](./instructions/instruction.md) 的要求。

如果您的歌词提交被驳回，请依照审核员的修改意见修改歌词，然后尝试再次提交，以下是常见的驳回原因：

- 单词时间错误，或偏移值过大；
- 矫枉过正：
  - 在不适用于部分效果的情况下，强行加入效果；
  - 为触发某些歌词效果，刻意错误打轴。
- 歌词行属性错误：
  - 未正确区分当前歌手处于辅助主唱，或是独自对唱演唱的状态，以至于错误设置了背景歌词和对唱歌词；
  - 除非歌词本身不为歌曲考虑使用对唱歌词，否则应该要根据当前演唱者主次关系设置正确的对唱歌词，在没有确切动机的情况下，不应将同一演唱者的同一演唱形式，设置出两种不同的歌词状态。
    > **演唱形式** 可以是主唱、说唱、和声等。

如果您认为您的歌词不存在审核员修改意见中的问题，请尝试再次提交并附上原因，以便审核员理解你的意图，或是请求其他审核员审核。

***

# 使用歌词数据库

## AMLL Player [推荐]

AMLL Player 是 Apple Music-like Lyrics 的本地客户端，可播放本地音乐和连接 WebSocket 服务端。[前往了解](https://github.com/Steve-xmh/applemusic-like-lyrics/actions/workflows/build-player.yaml)

AMLL Player 已内置歌词库搜索功能，导入本地歌曲后编辑歌词覆盖信息，即可从 AMLL TTML DB 搜索/导入歌词。

##  Apple Music-like Lyrics for BetterNCM [停止维护]

Apple Music-like Lyrics for BetterNCM 已内置本仓库歌词源，无需手动配置，仅需将歌词源 **AMLL TTML 逐词歌词数据库（多源聚合）** 置顶即可使用。

> 如果您依然希望使用网易云客户端作为您的播放器，同时能够继续使用本仓库中的歌词，那么请配合使用 [amll-bncm-ws-connector](https://github.com/Steve-xmh/amll-bncm-ws-connector) 与 AMLL Player。

### 镜像源

官方源出于部分原因，可能偶发无法搜索歌词、未搜索到歌词、歌词返回数据异常等问题，您可以使用以下镜像源，在插件设置 - `歌词源` - `从歌词源字符串添加` 中输入以下内容：

作者镜像源 [@Steve-xmh](https://github.com/Steve-xmh)

```text
61ba6770-f02f-11ef-a3ae-5396943709e6|AMLL%20TTML%20%E9%80%90%E8%AF%8D%E6%AD%8C%E8%AF%8D%E6%95%B0%E6%8D%AE%E5%BA%93%EF%BC%88stevexmh.net%20%E9%95%9C%E5%83%8F%EF%BC%89||ttml|https://amll-ttml-db.stevexmh.net/ncm/[NCM_ID]
```

### 社区镜像源

您也可以尝试由社区提供的镜像源，具体使用方法请自行在站内查阅，感谢 [@HelloZGY](https://github.com/cybaka520) 与 [@Luorix](https://github.com/LuorixDev) ！

[AMLL TTML DB 镜像站](https://amlldb.bikonoo.com/mirror.html) By [@HelloZGY](https://github.com/cybaka520)

```text
19cf30a0-6206-11f0-b2b3-0d580aff0f69|%E9%95%9C%E5%83%8F%E7%AB%99||ttml|https://amlldb.bikonoo.com/ncm-lyrics/[NCM_ID].ttml
```

[AMLL-TTML-DB 自动镜像站](https://amll.mirror.dimeta.top/) By [@Luorix](https://github.com/LuorixDev)

```text
06e48500-d086-11f0-bb6e-451fd0fc9216|Dimeta%E9%95%9C%E5%83%8F%E7%AB%99v1||ttml|https://amll.mirror.dimeta.top/api/db/ncm-lyrics/[NCM_ID].ttml
```

## UniLyric [适配最多]

UniLyric 不仅是一个全能的歌词转换器，同时可以作为 AMLL Player 的歌词发送端。其工作原理则是通过监听 [SMTC](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/integrate-with-systemmediatransportcontrols) 获取系统当前播放曲目的曲名、歌手名以及播放进度（如果 SMTC 发送方有设置的话），因此只要你的播放器支持 SMTC 就可以使用。并且 UniLyric 集成了包含 AMLL TTML DB 在内的多方歌词源并且能够自动搜索，可以说是目前最好用的词库搜索方和 AMLL Player 歌词发送端。 [前往了解](https://github.com/apoint123/Unilyric)

## 接入到其他项目

> [!TIP]
虽然这并非强制，但我们希望你在使用本歌词数据库时，能够在你的项目中加入一个指向本仓库或者衍生项目的链接或说明，或是展示每个歌词文件中的歌词作者信息（均可以在元数据中读取），以便更多人发现和建设本歌词数据库，这会给予我们莫大的帮助。

如果你想要接入本歌词数据库，可以通过对应各平台的文件夹，用您对应平台的音乐 ID 来获取歌词文件。

现阶段支持以下平台的歌词索引：

- [Netease Cloud Music - 网易云音乐](./ncm-lyrics) - [`ncm-lyrics/`](https://github.com/Steve-xmh/amll-ttml-db/tree/main/ncm-lyrics)
- [QQ Music - QQ 音乐](./qq-lyrics) - [`qq-lyrics/`](https://github.com/Steve-xmh/amll-ttml-db/tree/main/qq-lyrics)
- [Apple Music](./am-lyrics) - [`am-lyrics/`](https://github.com/Steve-xmh/amll-ttml-db/tree/main/am-lyrics)
- [Spotify](./spotify-lyrics) - [`spotify-lyrics/`](https://github.com/Steve-xmh/amll-ttml-db/tree/main/spotify-lyrics)

每个歌词文件均已自动生成不同格式的歌词文件，通过文件后缀名区分：

- `.ttml` - 原 TTML 歌词格式
- `.lrc` - LyRiC 歌词格式
- `.yrc` - 网易云音乐逐词歌词格式
- `.qrc` - QQ 音乐逐词歌词格式
- `.lys` - Lyricify Syllable 逐词歌词格式
- `.eslrc` - ESLrc 逐词歌词格式

您可以通过以下直链获取您对应平台音乐 ID 的歌词文件：

> `https://raw.githubusercontent.com/Steve-xmh/amll-ttml-db/refs/heads/main/[对应平台歌词文件夹]/[音乐ID].[后缀名]`

如果需要检索从建立数据库开始至今所有的歌词文件，可以访问 [raw-lyrics/](./raw-lyrics/) 文件夹，内部的文件以 `[提交 UNIX 时间戳]-[提交者 Github ID]-[8 位随机 ID].ttml` 命名。或者通过 [metadata/](./metadata) 文件夹中的 [`raw-lyrics-index.jsonl`](https://github.com/Steve-xmh/amll-ttml-db/raw/refs/heads/main/metadata/raw-lyrics-index.jsonl) 进行检索，里面包含了所有歌词文件的元数据，并且从上到下按照从旧到新的顺序依次列出。

同时，在每个平台文件夹下，还有一个存有基本信息的 `index.jsonl` 逐行存储了该平台下所属的所有歌词基本信息，以原始歌词文件顺序排列，也列出了所有历史歌词信息。

***

# 共享协议

本仓库的外来数据部分遵循原数据提供方的共享协议共享，提交者自主编写的部分使用 [CC0 1.0 共享协议](https://github.com/Steve-xmh/amll-ttml-db?tab=CC0-1.0-1-ov-file) 共享。

***

# 鸣谢

感谢各位对 AMLL 生态作品感兴趣的用户，也欢迎加入 AMLL 亲友团 QQ 群 `719423243` 参与讨论。

感谢所有为建设本仓库提供歌词的贡献者们！

[![贡献者头像画廊，点击可查阅](https://amll-ttml-db.stevexmh.net/contributors.png)](./CONTRIBUTORS.md)

