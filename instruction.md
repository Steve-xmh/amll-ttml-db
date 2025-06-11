# TTML 歌词相关内容介绍

## 第一章 关于 TTML

TTML (Timed Text Markup Language) 准确来说并不是一种歌词专用文件类型，而是一种字幕文件类型。它是一种基于 XML 的 W3C 在线媒体定时文本标准，旨在用于创作、转码或交换定时文本信息，目前主要应用于字幕（subtitling）和辅助字幕（captioning）功能 [^1]。在本词库中则用于显示逐字歌词。

### 第一节 TTML 格式

> [!TIP]
>
> 如果你只是想直接上手开始打轴，可以直接跳到 [第二章](#第二章-歌词规范（试行版）) 开始了解基本规范。本章仅对于 TTML 歌词文件的格式进行说明，以便于开发者或有特殊要求的打轴人员详细了解 TTML 格式的具体内容。

TTML 基于 XML 格式，其根节点为 `tt`，并引入 TTML 所需的命名空间：

```xml
<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" xmlns:amll="http://www.example.com/ns/amll" xmlns:itunes="http://music.apple.com/lyric-ttml-internal"></tt>
```

> [!IMPORTANT]
>
> 1. 请注意：TTML 是空格敏感的，并且为压缩格式，<u>如果使用**带有缩进、基于 XML 格式**进行格式化之后的文件，可能会导致解析错误</u>，如无法检测出空格、歌词开头出现多余空白等。
> 2. 由于 TTML 是基于 XML 的一种格式，因此其中的字符串应该为 XML 合法的，即：将字符 `<` `>` `&` `'` `"` 转移为实体引用之后的字符串。

根节点中包含两个部分：头部 (`head`) 和正文 (`body`)。

#### 部分一 头部 (head)

头部标签为 `head`，对于词库中的 `*.ttml` 文件，其中*有且仅有* `metadata` 一个子标签，并在其中包含代理与元数据两部分。

##### 第一小节 代理

**代理**用于分配每个角色所对应的标识，其格式如下：

```xml
<ttm:agent type="string" xml:id="string" />
```

其中，`type` 字段表示代理歌手的名称，`xml:id` 为该歌手的**唯一**标识符。

> **样例**
>
> ```xml
> <ttm:agent type="person" xml:id="v1" />
> <ttm:agent type="other" xml:id="v2" />
> ```

##### 第二小节 元数据

一个元数据的格式如下：

```xml
<amll:meta key="string" value="string" />
```

其中 `key` 的值可以重复，例如：当同一首歌曲有多个 ISRC 时，就会添加多个 `key` 为 `isrc` 的元数据。

> [!TIP]
>
> 以下元数据可参考 [AMLL Wiki](https://github.com/Steve-xmh/amll-ttml-tool/wiki/歌词元数据)
>
> - 歌曲信息
>   - 歌曲名称
>   - 歌曲的艺术家
>   - 歌曲的专辑名
> - 各平台歌曲 ID
>   - 网易云音乐 ID
>   - QQ音乐 ID
>   - Spotify 音乐 ID
>   - Apple Music 音乐 ID
> - ISRC
> - 歌词作者信息
>   - 逐词歌词作者 Github ID
>   - 逐词歌词作者 GitHub 用户名

以上列出的元数据在 [TTML TOOL](https://github.com/Steve-xmh/amll-ttml-tool) 中可以直接添加，但**元数据的种类并不局限于以上列出几项**。例如歌曲的编曲、作词以及版权等信息都能够以自定义 `key` 的方式写入元数据。

#### 部分二 正文 (body)

> **关于时间格式**
>
> TTML 文件中的时间格式为 `00:00.000`，分别对应*分钟*、*秒*、*毫秒*。

正文部分的标签为 `body`，`body` 中有属性 `dur`，其值为歌词持续时间。

`body` 中**有且仅有**一个 `div` 子标签，该标签有两个属性：`begin` 和 `end`，分别表示歌词的开始和结束时间。

> **样例**
>
> ```xml
> <body dur="03:00.000"><div begin="00:05.000" end="03:00.000"></div></body>
>    ```

`div` 标签中有数个子标签 `p`，每个 `p` 标签存储一个歌词行。

##### 第一小节 歌词行

一个 `p` 标签表示一个歌词行。其中有如下属性：

- `begin`：该行歌词开始时间，值为时间字符串；
- `end`：该行歌词结束时间，值为时间字符串；
- `ttml:agent`：该行歌词演唱者的**代理**，其值应被包含在 [被声明的代理](#第一小节-代理) 中；
- `itunes:key`：该行的编号，格式为 `L` 加该行的编号（从 1 开始）。

> **样例**
>
> ```xml
> <p begin="00:00.000" end="00:00.000" ttm:agent="v1" itunes:key="L1"></p>
> ```

一个歌词行中存在如下内容：歌词正文（由歌词音节组成）、歌词翻译行\*、歌词音译行\*、背景歌词行\*。（带 `*` 为非必需部分）

##### 第二小节 歌词音节

歌词音节分为 **DOM 节点音节**和**纯文本音节**。

其中纯文本音节为合法的 XML 字符串，而 DOM 节点音节为一个 `span` 标签，包裹一段合法的 XML 字符串表示音节文本，并包含以下属性：

- `begin`：音节开始时间
- `end`：音节结束时间

> **样例**
>
> ```xml
> <p begin="00:35.110" end="00:39.270" ttm:agent="v1" itunes:key="L13"><span begin="00:35.110" end="00:35.250">天</span><span begin="00:35.250" end="00:35.920">色</span><span begin="00:35.920" end="00:36.120">渐</span><span begin="00:36.290" end="00:36.920">暗</span> <span begin="00:37.340" end="00:37.480">人</span><span begin="00:37.480" end="00:37.850">群</span><span begin="00:37.850" end="00:38.130">看</span><span begin="00:38.130" end="00:38.330">不</span><span begin="00:38.330" end="00:38.440">出</span><span begin="00:38.530" end="00:38.920">悲</span><span begin="00:38.920" end="00:39.270">欢</span></p>
> ```
>
> 其中共 12 个音节，11 个 DOM 节点音节（`天` `色` `渐` `暗` `人` `群` `看` `不` `出` `悲` `欢`）和一个纯文本节点（` `，为一个空格）。

##### 第三小节 附加行

附加行包括：[翻译行](#壹-翻译行)、[音译行](#贰-音译行)、[背景行](#叁-背景行)。

###### 壹 翻译行

翻译行使用 `span` 包裹，内容为该行歌词翻译的合法 XML 字符串。翻译行存在以下两个属性：

- `ttm:role`：值固定为 `x-translation`
- `xml:lang`：翻译文本的语言，遵循 RFC 1766 标准 [^2]，*目前 AMLL 系列软件暂不支持多种翻译*。

> **样例一**
>
> ```xml
> <p begin="00:04.220" end="00:07.950" ttm:agent="v1" itunes:key="L1"><span ...>...</span><span ttm:role="x-translation" xml:lang="zh-CN">褪色的世界 早已被我习惯</span></p>
> ```
>
> **样例二**
>
> ```xml
> <p begin="00:21.400" end="00:23.870" ttm:agent="v1" itunes:key="L1"><span ...>...</span><span ttm:role="x-translation" xml:lang="en-US">Lower beings</span><span ttm:role="x-translation" xml:lang="ja-JP">劣等な生物たちよ</span></p>
> ```

###### 贰 音译行

音译行使用 `span` 包裹，内容为该行歌词音译的合法 XML 字符串。音译行只有一个属性 `ttm:role`，值固定为 `x-roman`。

> **样例**
>
> ```xml
> <p begin="00:18.810" end="00:23.100" ttm:agent="v1" itunes:key="L1"><span ...>...</span><span ttm:role="x-roman">yo ri sō fu ta ri ni ki mi ga overlap</span></p>
> ```

###### 叁 背景行

背景行一般用于标记和声，其内容和普通行相同，但有以下差别：

- 使用 `span` 标签代替 `p` 标签包裹；
- 添加属性 `ttm:role`，值固定为 `x-bg`；
- 没有 `ttm:agent` 和 `itunes:key` 两个属性；
- 第一个音节的文本前加正括号 `(`，最后一个音节的文本末加反括号 `)`；
- 背景行中不能再嵌套背景行。

>**样例**
>
>    ```xml
> <p begin="00:57.770" end="01:01.500" ttm:agent="v1" itunes:key="L10"><span ...>...</span><span ttm:role="x-bg" begin="01:00.440" end="01:02.220"><span begin="01:00.440" end="01:00.790">(在</span><span begin="01:00.790" end="01:01.210">乎</span><span begin="01:01.210" end="01:02.220">你)</span></span></p>
> ```
>

## 第二章 歌词规范（试行版）

### 第一节 歌词正文

- 不得出现违反法律法规的内容；
- 不得包含歌词内容以外的信息，例如作词、作曲等信息应当放在元数据部分；
  - ⚠ 如表演者为团体，且需要注明每一句的具体演唱者时，可在翻译行的开头注明该句的演唱者；
- 不得包含 Emoji 或颜文字等无关内容；
  - 版权方提供的原始歌词文本中包含这些内容时除外；
- 内容真实、准确；
  - ⚠ 出自游戏、电影等其他作品中的专有名词须进行校对以保证准确性；
- 易混淆或易拼写错误的专有名词须注明出处以便考证；
- 歌词行的结尾不得使用句号或逗号，但可以按需使用感叹号、问号和引号； [[来源]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea:~:text=歌词行的结尾不得使用句号或逗号，但可以按需使用感叹号、问号和引号。)
- 每行歌词都应遵循惯用的语法规则。 [[来源]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea:~:text=和引号。-,每行歌词都应遵循惯用的语法规则。,-其他可在歌词)

---

#### 部分一 英语限定
- 请遵循惯用的英文语法规则。专有名词必须首字母大写。此外，每行歌词中第一个单词必须首字母大写。
	- ✅ 有可信来源证明其原作者使用全大/小写文本的除外。
- 其他需要首字母大写的情况：
  - 与神和宗教相关的单词（宗教歌曲中的 God、You、Him、Your 等）
  - 缩略词大小写请遵循使用习惯（NASA、FBI 等）
  - 地理位置（East Coast、Southside 等）
  - 作品名称
  - 括号中第一个单词的首字母
  - 品牌名称

[[来源]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea)

---

#### 部分二 日语限定
- 尽可能在正文中使用方括号（「」『』）作为其他语言中的引号和单引号的代替；
- 不要把汉字对应的假名在文本中用括号列出来（包括<ruby>義訓<rt>ギクン</rt></ruby>）：
  
<table border="1">
<tr>
      <td>✅</td>
      <td>あの時こう……</td>
</tr>
<tr>
      <td>❌</td>
      <td>あの時（とき）こう…… </td>
</tr>
</table>

### 第二节 翻译和音译
- ⚠ 涉及政治敏感、有违人道主义的曲目不得提供歌词翻译；
	- ❌ 如果为上述情况的国语歌曲则**不予提交歌词**；
	- ⚠ 对于有可能 NSFW 的翻译，可以忠于原文，但请尽量注意尺度，否则可能因尺度过大不予通过；
- ⚠ 为避免可能的版权纠纷，使用 B站等视频平台中作品包含的原创翻译内容时，请标注来源；
- 不得出现违反法律法规及公序良俗的内容；
- 对于非国语歌词，请尽可能提供真实、准确的翻译和音译；
- 原则上禁止在翻译中插入 Emoji 或颜文字等无关内容，除非翻译作品的版权方也是这么做的。


### 第三节 排版
- 尽可能按照单个文字或单个音节制作逐字/逐音节歌词；
- 在机器人处理后的文件中，不允许出现空格包含在音节内**首尾**的情况；
- 不允许包含空白行；
- 在不影响时间轴精确度的情况下，可以将标点符号作为单独的单词打轴；
- 合理使用对唱/背景视图：
  - 背景人声的歌词应单独为一行，放在主唱人声歌词的下一行并标记为背景行；
  	- （非强制要求）不应因背景行而延长主行时间轴的持续时间；
  - 如果不确定何时换行或分段，请参考以下划分依据： [[来源]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea)
  	- 明确的 chorus（副歌）、verse（主歌）、intro（前奏）、bridge（桥段）或 Hook
  	- 歌曲节拍速度发生变化
  	- 艺人对歌词的演绎方式发生变化（例如，从歌唱切换成说唱）
- 一般不允许多行歌词的时间轴重叠，但在不影响歌词准确的情况下，下面这些情况是例外：
  - 对唱中两位或多位艺人进行合唱或重唱；
  - 上下文具有关联性，为了叙事或表达要求而特意设置的，例如**完整的一句被拆分为了语法缺失的两句**或**上下两句互文、对仗**；
  - 由于混音或编曲使两行歌词的时间轴有重叠部分。

> [!WARNING]
>
> 对于背景行，如果使用 TTML TOOL 进行歌词的制作，请不要在第一个音节的开头和最后一个音节的末尾添加括号，这是因为 TTML TOOL 会自行处理括号部分，如果再手动添加，则会导致歌词中出现多余括号。
>
> 例如以下背景行在 TTML TOOL 中显示为：
>
> > <kbd>In</kbd> <kbd>空格x1</kbd> <kbd>your</kbd> <kbd>空格x1</kbd> <kbd>heart</kbd>
>
> 那么导出时为：
>
> ```xml
> <span ttm:role="x-bg" begin="02:23.620" end="02:25.690"><span begin="02:23.620" end="02:24.100">(In</span> <span begin="02:24.100" end="02:24.380">your</span> <span begin="02:24.380" end="02:25.690">heart)</span></span>
> ```
>
> 但如果在 TTML TOOL 中手动再添加一次括号：
>
> > <kbd>(In</kbd> <kbd>空格x1</kbd> <kbd>your</kbd> <kbd>空格x1</kbd> <kbd>heart)</kbd>
>
> 那么导出时就会变为：
>
> ```xml
> <span ttm:role="x-bg" begin="02:23.620" end="02:25.690"><span begin="02:23.620" end="02:24.100">((In</span> <span begin="02:24.100" end="02:24.380">your</span> <span begin="02:24.380" end="02:25.690">heart))</span></span>
> ```
>
> 成为一种不合规的形式。

#### 部分一 英语限定
- 不得将空格包含在单词首尾

<table border="1">
<tr>
      <td>❌</td>
      <td>This \is \a \lyric</td>
</tr>
<tr>
      <td>❌</td>
      <td>This\ is\ a\ lyric</td>
</tr>
<tr>
      <td>✅</td>
      <td>This\ \is\ \a\ \lyric</td>
</tr>
<tr>
      <td>✅</td>
      <td>Thi\s i\s a\ \ly\ri\c</td>
</tr>
</table>


- 不允许使用一个以上的空格来分隔单词

### 第四节 HOYO-MiX
- 在以上要求的基础之上，要求将尽可能多地区的元数据添加到文件中；
  - 当前，Apple 和 Spotify 分为下面这些分区，请按照这些分区逐一查找并添加元数据：
    - 简中区（Spotify 无简中区）；
    - 繁中区；
    - 韩语区；
    - 日语区；
    - 英语区。
- 必须是逐字歌词；
- 必须适配艺人演唱时的各类效果。

### 第五节 其他
- 请善用结束时间来让歌词播放器自动生成间奏区域，不要为了演出效果强制改变为错误的时间轴，例如为了不触发间奏强行延后上一句结束时间和提早下一句开始时间或为了触发间奏强行提前上一行结束时间和延后下一句开始时间；
- 歌曲作者等信息请使用 AMLL TTML Tool 的元数据功能添加（包括但不限于预设的字段）；
- 提交时标题中的「歌词提交/修正」尽量根据实际提交内容改为「歌词修正」或「歌词提交」；
- 如果是对已有歌词的修正，请在补充说明处写明修改原因提供给审核核对，否则将被退回。

## 附录Ⅰ 参考页面

[^1]: [Timed Text Markup Language - Wikipedia](https://en.wikipedia.org/wiki/Timed_Text_Markup_Language)

[^2]: [Information on RFC 1766 » RFC Editor](https://www.rfc-editor.org/info/rfc1766)
