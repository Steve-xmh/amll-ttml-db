# TTML 歌词文件规范

### 1. 目的与概述

本文档旨在为 AMLL TTML Database 定义一套标准的 TTML (Timed Text Markup Language) 文件格式。所有提交到本仓库的歌词文件都**必须**遵循此规范，以便正确解析和存储。

本文档基于 W3C TTML1 标准，并针对 Apple Music 格式进行了扩展。

> [!WARNING]
> 为了确保可读性，下列的 TTML 片段示例经过格式化。但上传 TTML 文件时，**不建议**格式化。详细信息请参考第 6 节。

> [!CAUTION]
> 这份规范目前只对实验性提交流程生效。
---

### 2. 文件基本结构

每个 TTML 文件都必须是一个合法的 XML 文档，并包含以下基本结构和命名空间声明。


```xml
<?xml version="1.0" encoding="UTF-8"?>
<!-- 必须为 UTF-8 编码，且文件不得包含字节顺序标记 (BOM)。 -->
<tt xmlns="http://www.w3.org/ns/ttml"
    xmlns:ttm="http://www.w3.org/ns/ttml#metadata"
    xmlns:itunes="http://itunes.apple.com/lyric-ttml-extensions" 
    xmlns:amll="http://www.example.com/ns/amll"
    xml:lang="en"
    itunes:timing="Word">

    <head>
        <!-- 元数据 -->
    </head>

    <body dur="00:15.500">
        <!-- 歌词内容 -->
    </body>
</tt>
```

* **XML 声明**: TTML 文件中不得包含字节顺序标记 (BOM)。
* **根元素**: 必须是 `<tt>`。
* **命名空间**:
    * `xmlns`, `xmlns:ttm` 是标准 TTML 必需的。
    * `xmlns:itunes` 可选，用于 Apple Music 特定的属性，如 `itunes:timing` 和 `itunes:song-part`。
    * `xmlns:amll` 用于 AMLL 的元数据。
* **`xml:lang`**: 建议填写，在 `<tt>` 标签上指定歌词的主要语言代码 (例如 `ja` 代表日语, `en` 代表英语)。
* **`itunes:timing`**: 可选，用于声明逐行或逐字歌词。
    * `word`: 逐字歌词。
    * `line`: 逐行歌词。
    若不指定该属性，机器人将根据文件内容自动判断计时模式：如果歌词行 `<p>` 内包含了带时间戳的 `<span>` 标签，则按**逐字歌词**处理；否则，按**逐行歌词**处理。
* **`body` 元素**: 用于包含所有歌词行 (`<p>`) 和结构块 (`<div>`)。
    * **`dur`**: **可选，并不会影响时长计算。主要用于参考。**。如果包含，其值**必须大于或等于**文件中最后一个时间戳的结束时间。所有内部元素的时间码都不得超过此 `dur` 值。

    ```xml
    <body dur="00:04:15.500">
        </body>
    ```

---

### 3. 元数据

所有元数据都应放置在 `<head><metadata>...</metadata></head>` 标签内。

有多个值的，应为**每个值**创建一个标签。

#### 3.1. 歌曲与演唱者

使用标准 TTML 标签来定义歌曲基础信息和演唱者。

* **歌曲名**: 可以使用 `<ttm:title>`，最后会转换为 `musicName`。**不要**同时在 `<ttm:title>` 和 `musicName` 标签添加相同的值。

* **演唱者**: 使用 `<ttm:agent>` 定义所有演唱者。

    - `type` 属性指明类型: `person` (独唱), `group` (合唱，一般使用 `v1000`), `other` (其他)。

    - `xml:id` 为每位演唱者提供一个唯一的引用 ID (建议使用 `v1`, `v2`, `v1000`, ...)。

    - `<ttm:name>` 标签内提供可选的演唱者全名。

* **其他 `<ttm:...>` 标签**: 最后会转换为自定义的 AMLL 元数据标签。

    - 例如，`<ttm:copyright>一些版权信息</ttm:copyright>` 会被转换为 `<amll:meta key="copyright" value="一些版权信息"/>`。

#### 3.2. AMLL 元数据

使用 `<amll:meta>` 标签提供歌曲的核心信息。

* **歌曲名**: `key="musicName"`，**必填**
* **艺人名**: `key="artists"`，**必填**
* **专辑名**: `key="album"` (如果是单曲，专辑名应与歌曲名相同)，**必填**
* **ISRC号码**: `key="isrc"`

为了使歌词能够关联到各大音乐平台，**必须至少提供一个**平台 ID。

* **网易云音乐**: `key="ncmMusicId"`
* **QQ 音乐**: `key="qqMusicId"`
* **Spotify**: `key="spotifyId"`
* **Apple Music**: `key="appleMusicId"`

可以使用 AMLL 元数据标记歌词作者，例如：

* **逐词歌词作者 Github ID**: `key="ttmlAuthorGithub"`
* **逐词歌词作者 GitHub 用户名**: `key="ttmlAuthorGithubLogin"`

请参阅 [AMLL Wiki](https://github.com/Steve-xmh/amll-ttml-tool/wiki/%E6%AD%8C%E8%AF%8D%E5%85%83%E6%95%B0%E6%8D%AE) 了解更多信息。

```xml
<head>
    <metadata>
        <!-- Apple Music 元数据 -->
        <!-- 仅作为示例，你不应该添加两个歌曲名标签。 -->
        <ttm:title>歌曲名</ttm:title>
        <ttm:agent type="person" xml:id="v1">
            <ttm:name type="full">艺人A</ttm:name>
        </ttm:agent>
        <ttm:agent type="person" xml:id="v2">
            <ttm:name type="full">艺人B</ttm:name>
        </ttm:agent>
        <ttm:agent type="group" xml:id="v1000" />
        
        <!-- AMLL 元数据 -->
        <amll:meta key="musicName" value="歌曲名" />
        <amll:meta key="musicName" value="可能的第二个歌曲名" />
        <amll:meta key="artists" value="艺术家名" />
        <amll:meta key="artists" value="可能的第二个艺术家名" />
        <amll:meta key="album" value="专辑名"/>
        <amll:meta key="album" value="可能的第二个专辑名"/>
        <amll:meta key="ncmMusicId" value="123456789"/>
        <amll:meta key="spotifyId" value="123456789"/>
        <amll:meta key="ttmlAuthorGithub" value="123456789"/>
        <amll:meta key="ttmlAuthorGithubLogin" value="你的 Github 用户名"/>
    </metadata>
</head>
```


---

### 4. 计时模式

时间码必须严格遵循以下规则：

1. **有效性**: `begin` 时间必须早于 `end` 时间。

2. **嵌套规则**: 子元素的时间码必须完全包含在父元素的时间码之内。
    - 例: `<span>` 的时间范围必须在 `<p>` 的时间范围内；`<p>` 的时间范围必须在 `<div>` 的时间范围内。

3. **范围**: 所有时间码必须在歌曲的总时长 (`body` 的 `dur` 属性) 之内。

4. **重叠**: 不同演唱者的 `<p>` 或 `<span>` 时间码可以重叠，但同一演唱者的 `<p>` 或 `<span>` 时间码不能重叠。因混音而使时间重叠的情况除外。

#### 4.1. 逐字歌词

当 `itunes:timing="Word"` 时：

* 每一行歌词在一个 `<p>` 标签内。
* 每个音节都必须包裹在**带有 `begin` 和 `end` 属性的 `<span>` 标签**中。
* 单词之间的空格处理方式，请严格遵循下面的 **4.1.1. 空格处理规则**。

```xml
<p begin="00:01.000" end="00:03.500">
    <span begin="00:01.000" end="00:01.800">ただ一人</span>
    <span begin="00:02.000" end="00:03.500">迷い込む旅の中で</span>
</p>
```

##### 4.1. 空格处理规则

在逐词模式下，单词（音节）之间的空格是**有意义的字符**，必须被显式表示。机器人会识别以下几种表示方式，并以后两种为标准。

| 方法                     | 示例                                               | 规范性                         | 说明                                                                                                                  |
| :----------------------- | :------------------------------------------------- | :----------------------------- | :-------------------------------------------------------------------------------------------------------------------- |
| **空格在 `<span>` 内部** | `<span begin="00:01.0" end="00:02.0">word </span>` | **不规范（对于非格式化文件）** | 机器人会自动将音节的前导或尾随空格提取出来。                                                                          |
| **空格在 `<span>` 外部** | `<span begin="00:01.0" end="00:02.0">word</span> ` | **最规范**                     | 空格作为一个独立的文本节点存在于两个 `<span>` 标签之间。                                                              |
| **独立的空格 `<span>`**  | `<span begin="00:00.000" end="00:00.000"> </span>` | **允许**                       | 允许为空格创建独立的 `<span>` 标签。建议将其 `begin` 和 `end` 时间设为前一个音节的 `end` 时间，或统一设为 `00:00.000` |

如果你提交格式化后的 TTML 文件，**强烈建议**将空格直接写入到 `span` 内，这样可以避免一些奇奇怪怪的问题。详情请查看第六节。

##### 4.2. 自动分词

为日语和韩语歌曲打轴时，你可能会将多个字符放进一个音节来打轴（例如拗音等）。自动分词可以帮你平分这些包含了多个字符的音节，将这个长 `<span>` 拆分为多个独立的、每个只包含单个字符的 `<span>`，并按字符数按比例分配时间。

自动分词同样适用于直接从 Apple Music 上获取的歌词（Apple Music 会将多个持续时长相近的 CJK 字符合并为一个音节），只需要添加必要的元数据，再开启自动分词功能即可。

通常建议每个 `<span>` 内只有**一个** CJK 字符。多个 CJK 字符可能会导致不自然的辉光效果。

**输入示例：**

```xml
<span begin="10.0s" end="12.0s">你好世界</span>
```

**启用自动分词后的输出结果：**

```xml
<span begin="10.000" end="10.500">你</span>
<span begin="10.500" end="11.000">好</span>
<span begin="11.000" end="11.500">世</span>
<span begin="11.500" end="12.000">界</span>
```

自动分词功能还会尝试将一个英文单词按照音节拆分，例如 `analyse` -> `an-a-lyse`。**如果这不是你期望的行为，请不要开启自动分词功能**

**标点符号权重**用来控制拆分标点符号时应该为其分配多少时长，一般情况下不需要修改它，除非你确实希望它持续得更短或更长。

#### 4.3. 逐行歌词

当 `itunes:timing="Line"` 时，机器人只解析整行歌词的时间戳，并忽略内部 `<span>` 的时间戳信息（实际上也不应该有）。

* 每行歌词在一个**带有 `begin` 和 `end` 属性的 `<p>` 标签**内。
* 该行所有的文本内容直接放在 `<p>` 标签内。可以为了添加翻译等信息使用 `<span>`，但这些 `<span>` 的 `begin` / `end` 属性会被忽略。

```xml
<p begin="00:01.000" end="00:03.500">一行歌词</p>
```

如果要上传逐行歌词，建议开启“**这是逐行歌词**”选项。虽然不开也可以识别，但开启可以确保不会误判为逐字歌词。

#### 4.4. 时间戳格式

本文档中所有的时间值（如 `begin`, `end`, `dur` 属性的值）都**必须**遵循以下格式之一。

##### **时钟时间**

推荐基于时钟的 `HH:MM:SS.fff` 格式。

* **`HH`**: 小时，两位数字，可选。
* **`MM`**: 分钟，两位数字，当 `HH` 存在时必填。
* **`SS`**: 秒，两位数字，必填。
* **`.fff`**: 毫秒，可选，小数点后可跟1至3位数字。

**格式的省略规则：**

* 可以省略小时部分 `HH:`，格式变为 `MM:SS.fff`。
* 可以同时省略小时 `HH:` 和分钟 `MM:` 部分，格式变为 `SS.fff`。
* 可以省略毫秒部分 `.fff`。

> [!CAUTION]
> **关于分钟和秒的取值范围**
>
> * 当时间戳包含冒号 (`:`) 时（即格式为 `HH:MM:SS` 或 `MM:SS`），分钟 `MM` 和秒 `SS` 的值都**必须小于 60**。例如，`01:75.000` 是**无效**格式。
> * 当时间戳不包含冒号时（例如 `95.000`），秒的值可以大于等于 60。

##### **毫秒部分的解析规则**

解析器会自动补全毫秒部分的小数点后位数：

* **1 位数字**: 代表十分之一秒。例如 `15.1` 会被解析为 `15` 秒 `100` 毫秒。
* **2 位数字**: 代表百分之一秒。例如 `15.12` 会被解析为 `15` 秒 `120` 毫秒。
* **3 位数字**: 代表毫秒。例如 `15.123` 会被解析为 `15` 秒 `123` 毫秒。

##### **秒值**

可以直接提供以秒为单位的时间值，并以 `s` 作为后缀。该值可以是整数或浮点数。

* **示例**: `12.3s` 表示 `12300` 毫秒。`90s` 表示 `90000` 毫秒。

##### **有效格式示例汇总**

| 分类             | 格式           | 示例           | 解析后的毫秒值 |
| :--------------- | :------------- | :------------- | :------------- |
| **完整格式**     | `HH:MM:SS.fff` | `00:02:35.500` | `155500`       |
|                  | `HH:MM:SS.f`   | `00:02:35.5`   | `155500`       |
|                  | `HH:MM:SS`     | `00:02:35`     | `155000`       |
| **省略小时**     | `MM:SS.ff`     | `02:35.55`     | `155550`       |
|                  | `MM:SS`        | `02:35`        | `155000`       |
| **仅秒**         | `SS.fff`       | `35.123`       | `35123`        |
|                  | `SS`           | `35`           | `35000`        |
|                  | `SS` (超过60)  | `95`           | `95000`        |
| **`s` 后缀格式** | `f.f...s`      | `15.8s`        | `15800`        |
|                  | `fs`           | `15s`          | `15000`        |
---

### 5. 歌词内容和结构

#### 5.1. 歌词组成部分

使用 `<div>` 标签来分割歌曲的不同部分（如主歌、副歌），并通过 `itunes:song-part` 属性来标记。这是可选的内容。

* `itunes:song-part` 属性可以指定为任意值，但我们建议使用以下值：
    - `Verse` (主歌), 
    - `Chorus` (副歌), 
    - `PreChorus` (预副歌), 
    - `Bridge` (桥段), 
    - `Intro` (前奏), 
    - `Outro` (尾奏), 
    - `Refrain` (叠句), 
    - `Instrumental` (器乐)。

* `<div>` 块可以拥有 `begin` 和 `end` 时间码，其时间范围必须能完全包含内部所有子元素的时间。

```xml
<body>
    <div begin="00:10.000" end="00:25.000" itunes:song-part="Verse">
        <p begin="..." end="...">...</p>
        <p begin="..." end="...">...</p>
    </div>
    <div begin="00:25.500" end="00:40.000" itunes:song-part="Chorus">
        <p begin="..." end="...">...</p>
    </div>
</body>
```

#### 5.2. 歌词行、字词与演唱者

* **行 (`<p>`)**: `<p>` 标签用于放置歌词中的每一行。应使用 `<p>` 分隔歌词行，而不是 `<br>`。

* **字词 (`<span>`)**: 在逐字歌词中，`<span>` 用于标记单个字词或音节的时间。

* **演唱者 (`ttm:agent`)**: 在 `<p>` 标签上使用 `ttm:agent` 属性，并通过在 `<head>` 中定义的 `xml:id` (如 `v1`) 来指明演唱者。

* **行号 (`itunes:key`)**: 用于标记歌词行的唯一编号。其格式为 `L` 加上从 1 开始的行号 (例如 `L1`, `L2`, ...)。行号**必须**是连续且递增的，即使在不同的 `<div>` 块之间。

> [!WARNING]
> 即使是单人演唱的歌曲，也应为 `<p>` 标签添加 `ttm:agent="v1"`，并定义 "v1" agent。

#### 5.3. 多语言与背景支持

可以在主歌词行内嵌套 `<span>` 来提供翻译、罗马音和背景人声。

> [!CAUTION]
> AMLL 全系目前还不支持多翻译和多罗马音。不建议现在提交多翻译和多罗马音歌词。

* **翻译**: 使用 `<span ttm:role="x-translation" xml:lang="语言代码">...</span>`。
* **罗马音**: 使用 `<span ttm:role="x-roman" xml:lang="语言-Latn" xml:scheme="罗马音方案">...</span>`。
* **背景人声**: 使用 `<span ttm:role="x-bg" begin="..." end="...">...</span>`。如果背景人声出现在主唱人声之前，建议将 `<span ttm:role="x-bg">` 标签放在主唱人声的 `<span>` 标签之前。否则，请将 `<span ttm:role="x-bg">` 标签放在 `<p>` 标签的末尾。建议使用半角括号将背景人声文本包裹起来。机器人也会自动添加括号（如果没有）。不建议添加两个或更多括号，尽管词库的机器人可以正确处理，但其它解析器不一定。

```xml
<p begin="00:25.100" end="00:32.500" itunes:key="L1" ttm:agent="v1">
    <!-- 主歌词 (逐字计时) -->
    <span begin="00:25.100" end="00:25.800">君</span>
    <span begin="00:25.900" end="00:26.100">の</span>
    <span begin="00:26.300" end="00:27.600">知らない</span>
    <span begin="00:27.700" end="00:29.100">物語</span>

    <!-- 翻译内容 -->
    <span ttm:role="x-translation" xml:lang="zh-CN">你所不知道的物语</span>
    <span ttm:role="x-translation" xml:lang="en">The Story You Don't Know</span>

    <!-- 罗马音: 使用 xml:scheme 属性来标记不同的方案 -->
    <span ttm:role="x-roman" xml:lang="ja-Latn" xml:scheme="hepburn">kimi no shiranai monogatari</span>
    <span ttm:role="x-roman" xml:lang="ja-Latn" xml:scheme="kunrei">kimi no siranai monogatari</span>

    <!-- 背景人声 -->
    <span ttm:role="x-bg" begin="00:30.500" end="00:32.500">
        <!-- 背景人声的主歌词 -->
        <span begin="00:30.500" end="00:31.500">(秘密</span>
        <span begin="00:31.600" end="00:32.500">だよ)</span>
        
        <!-- 背景人声的翻译 -->
        <span ttm:role="x-translation" xml:lang="zh-CN">是秘密哦</span>
        <span ttm:role="x-translation" xml:lang="en">It's a secret</span>
        
        <!-- 背景人声的罗马音 -->
        <span ttm:role="x-roman" xml:lang="ja-Latn" xml:scheme="hepburn">himitsu da yo</span>
        <span ttm:role="x-roman" xml:lang="ja-Latn" xml:scheme="kunrei">himitu da yo</span>
    </span>
</p>
```

#### 5.4 Apple Music 样式翻译

除了在 `5.3` 中描述的内嵌辅助歌词（例如 `<span ttm:role="x-translation">...</span>`）外，机器人也兼容在 `<head>` 中定义的 Apple Music 样式翻译和音译。

> [!CAUTION]
> 当两种格式同时存在时，机器人会从 `<head>` 中获取 Apple Music 样式的翻译，并将其追加到该行歌词的翻译列表中。**这可能会导致双重翻译**。
> 为避免重复，请确保同一语言的翻译只出现在一种格式中。例如，如果 `<head>` 中定义了 `xml:lang="zh-CN"` 的翻译，则 `<body>` 的对应行内就不应再包含 `xml:lang="zh-CN"` 的翻译 `<span>`。

#### **结构说明**

1.  **位置**: 所有 Apple Music 样式的辅助轨道数据都必须置于 `<head><metadata>...</metadata></head>` 内部。
2.  **主容器**: 需要一个 `<iTunesMetadata>` 标签作为所有 Apple Music 特定元数据的容器。
3.  **轨道类型容器**:
      * **翻译**: 使用 `<translations>` 标签包裹。
      * **音译**: 使用 `<transliterations>` 标签包裹。
4.  **语言块**:
      * 在 `<translations>` 或 `<transliterations>` 内部，每个 `<translation>` 或 `<transliteration>` 块代表一种语言的轨道。
      * 每个 `<translation>` 代表一种语言的翻译，且**必须**包含以下属性：
        * `type="翻译类型"`，可以为 `subtitle` 或 `replacement`。`subtitle` 适用于大部分翻译内容，`replacement`一般用于简繁中文转换。
        * `xml:lang="语言代码"` (例如: `zh-Hans-CN`)
5.  **文本链接**:
      * 在每个语言块内部，内容由一个或多个 `<text>` 标签承载。
      * 通过 `for` 属性将内容与歌词行进行关联，其值**必须**与 `<body>` 中对应 `<p>` 标签的 `itunes:key` 值完全一致 (例如, `for="L1"`)。

#### **内容格式**

`<text>` 标签内部的内容可以是以下两种格式之一：

  * **逐行**: 直接包含翻译或音译的纯文本。

    ```xml
    <text for="L1">This is a line-by-line translation</text>
    ```

  * **逐字**: 包含一个或多个带 `begin` 和 `end` 属性的 `<span>` 标签。

    ```xml
    <text for="L2">
      <span begin="10.0s" end="10.5s">A </span>
      <span begin="10.5s" end="11.0s">syllable-timed </span>
      <span begin="11.0s" end="11.8s">translation</span>
    </text>
    ```

#### **背景人声**

你可以在 `<text>` 标签内使用 `<span ttm:role="x-bg">` 来给背景人声提供翻译或音译。

  * 对于**逐行**的背景人声辅助歌词，在 `<text>` 内使用带 `ttm:role="x-bg"` 的 `<span>`。
  * 对于**逐字**的背景人声辅助歌词，在 `ttm:role="x-bg"` 的 `<span>` 内部再嵌套带时轴的 `<span>`。

-----

#### **示例 1：逐行翻译**

以下示例展示了如何在头信息中定义简体中文翻译，并将其链接到正文中的歌词行。

**`<head>` 部分的定义:**

```xml
<head>
    <metadata>
        <iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal">
            <translations>
                <translation type="subtitle" xml:lang="zh-Hans-CN">
                    <text for="L23">黄金首饰 闪亮耀眼</text>
                    <text for="L24">冰镇草莓香槟</text>
                    <text for="L25">你走运了 这正是我喜欢的</text>
                </translation>
            </translations>
        </iTunesMetadata>
    </metadata>
</head>
```

**`<body>` 部分对应的歌词行:**

```xml
<body>
    ...
    <div itunes:songPart="Chorus">
        <p begin="45.404" end="48.709" itunes:key="L23" ttm:agent="v1">
            <span begin="45.404" end="45.755">Gold </span>
            <span begin="45.755" end="46.696">jewelry </span>
            <span begin="46.696" end="47.627">shining </span>
            <span begin="47.627" end="47.979">so </span>
            <span begin="47.979" end="48.709">bright</span>
        </p>
        <p begin="48.739" end="52.311" itunes:key="L24" ttm:agent="v1">
            <span begin="48.739" end="50.290">Strawberry </span>
            <span begin="50.290" end="51.226">champagne </span>
            <span begin="51.226" end="51.584">on </span>
            <span begin="51.584" end="52.311">ice</span>
        </p>
        <p begin="52.320" end="54.350" itunes:key="L25" ttm:agent="v1">
            <span begin="52.320" end="52.826">Lucky </span>
            <span begin="52.826" end="53.090">for </span>
            <span begin="53.090" end="53.300">you, </span>
            <span begin="53.300" end="53.484">that's </span>
            <span begin="53.484" end="53.732">what </span>
            <span begin="53.732" end="53.918">I </span>
            <span begin="53.918" end="54.350">like</span>
        </p>
    </div>
    ...
</body>
```

-----

#### **示例 2：逐字翻译与音译**

**`<body>` 部分的歌词行:**

```xml
<body>
  <p begin="10.0s" end="12.0s" itunes:key="L1">
      <span begin="10.0s" end="10.8s">두렵지는 않아</span>
      <span ttm:role="x-bg">
          <span begin="11.0s" end="11.8s">(흥미로울 뿐)</span>
      </span>
  </p>
</body>
```

**`<head>` 部分对应的辅助轨道定义:**

```xml
<head>
  <metadata>
    <iTunesMetadata>
      <translations>
        <translation xml:lang="en-US">
          <text for="L1">
            I'm not afraid
            <span ttm:role="x-bg">(Just interesting)</span>
          </text>
        </translation>
      </translations>
      <transliterations>
        <transliteration xml:lang="ko-Latn">
          <text for="L1">
            <span begin="10.0s" end="10.8s">duryeopjineun ana</span>
            <span ttm:role="x-bg">
              <span begin="11.0s" end="11.4s">heungmiroul </span>
              <span begin="11.4s" end="11.8s">ppun</span>
            </span>
          </text>
        </transliteration>
      </transliterations>
    </iTunesMetadata>
  </metadata>
</head>
```

---

### 6. 空格与格式化规范

#### 6.1 格式化支持

你可以使用格式化工具来格式化 TTML 文件。但 `<span>` 标签后的所有空格均会丢失（只要 `<span>` 标签后包含换行符）。

若要保留这些空格，请将空格直接写入到 `<span>` 标签内。例如：

```xml
<p begin="45.404" end="48.709" itunes:key="L23" ttm:agent="v1">
    <span begin="45.404" end="45.755">Gold </span>
    <span begin="45.755" end="46.696">jewelry </span>
    <span begin="46.696" end="47.627">shining </span>
    <span begin="47.627" end="47.979">so </span>
    <span begin="47.979" end="48.709">bright </span>
</p>
```

或者，将两个应该以空格分割的音节写成一行：

```xml
<span begin="10s" end="11s">word1</span> <span begin="12s" end="13s">word2</span>
```

使用实验性提交流程生成格式化的 TTML 文件时，默认会将空格直接写入到 `<span>` 标签内。

#### 6.2 空格处理

我们会自动规范化歌词文本中的空格，将多个连续的空格（包括换行符、制表符等）合并为一个标准的半角空格，并移除 `<span>` 内部的空格。

在**逐字模式**下，词间的空格至关重要。尽管格式化是被允许的，但我们依然**强烈建议**遵循 **4.1.1. 空格处理规则** 中定义的最佳实践来表示单词间的空格。

---

### 7. 语言代码规范 (BCP-47)

本文档中所有用于指定语言的 `xml:lang` 属性，其值 **必须** 遵循 IETF 的 **BCP-47** 标准。

BCP-47 是用于标识人类语言的国际标准代码。它通常由一系列用连字符 (`-`) 分隔的子标签组成，用以表示语言、文字、地区等信息。

> [!TIP]
> 你可以通过 [IANA 语言子标签注册表](https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry) 查询所有有效的语言代码。

#### 常见示例

* **基本语言标签**
    * `ja`: 日语
    * `en`: 英语
    * `ko`: 韩语

* **语言-文字标签**
    * `zh-Hans`: 简体中文
    * `zh-Hant`: 繁体中文
    * `ja-Latn`: 日语罗马音

* **语言-区域标签**
    * `en-US`: 美国英语
    * `en-GB`: 英国英语

* **语言-文字-区域标签**
    * `zh-Hans-CN`: 中国大陆地区使用的简体中文

#### 应用范围

此规范适用于文件中所有出现的 `xml:lang` 属性，包括但不限于：

* **根元素**: `<tt xml:lang="...">`
* **行内翻译**: `<span ttm:role="x-translation" xml:lang="...">`
* **行内罗马音**: `<span ttm:role="x-roman" xml:lang="...">`
* **头信息翻译**: `<translation type="subtitle" xml:lang="...">`
