# TTML 歌词文件规范

### 1. 目的与概述

本文档旨在为 AMLL TTML Database 定义一套标准的 TTML (Timed Text Markup Language) 文件格式。所有提交到本仓库的歌词文件都**必须**遵循此规范，以便正确解析和存储。

本文档基于 W3C TTML1 标准，并针对 Apple Music 格式进行了扩展。

> [!CAUTION]
> 为了确保可读性，下列的 TTML 片段示例经过格式化。但上传 TTML 文件时，**不允许**格式化。

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
* **`xml:lang`**: 可选，在 `<tt>` 标签上指定歌词的主要语言代码 (例如 `ja` 代表日语, `en` 代表英语)。
* **`itunes:timing`**: 可选，用于声明逐行或逐字歌词。若不指定该属性，默认为逐字歌词。若 TTML 文件中没有任何逐字音节信息（即所有文本均直接包含在 `<p>` 标签中），会自动判断为逐行歌词。
* **`body` 元素**: 用于包含所有歌词行 (`<p>`) 和结构块 (`<div>`)。
    * **`dur`**: **必填**。用于定义歌词内容的总时长。其值应约等于文件中最后一个时间戳的结束时间。所有内部元素的时间码都不得超过此 `dur` 值。

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

* **歌曲名**: 可以使用 `<ttm:title>`，最后会转换为 `musicName`。

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

##### 4.1.1. 空格处理规则

在逐词模式下，单词（音节）之间的空格是**有意义的字符**，必须被显式表示。机器人会识别以下几种表示方式，并以后两种为标准。

| 方法 | 示例 | 规范性 | 说明 |
| :--- | :--- | :--- | :--- |
| **空格在 `<span>` 内部** | `<span begin="00:01.0" end="00:02.0">word </span>` | **不规范，会自动修正** | 机器人会自动将音节的前导或尾随空格提取出来。 |
| **空格在 `<span>` 外部** | `<span begin="00:01.0" end="00:02.0">word</span> ` | **最规范** | 空格作为一个独立的文本节点存在于两个 `<span>` 标签之间。 |
| **独立的空格 `<span>`** | `<span begin="00:00.000" end="00:00.000"> </span>` | **允许** | 允许为空格创建独立的 `<span>` 标签。建议将其开始和结束时间均设为 `0`。 |

#### 4.2. 逐行歌词

当 `itunes:timing="Line"` 时，机器人只解析整行歌词的时间戳，并忽略内部 `<span>` 的时间戳信息（实际上也不应该有）。

* 每行歌词在一个**带有 `begin` 和 `end` 属性的 `<p>` 标签**内。
* 该行所有的文本内容直接放在 `<p>` 标签内。可以为了添加翻译等信息使用 `<span>`，但这些 `<span>` 的 `begin` / `end` 属性会被忽略。

```xml
<p begin="00:01.000" end="00:03.500">一行歌词</p>
```

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

* **行 (`<p>`)**: <p> 标签用于放置歌词中的每一行。应使用 `<p>` 分隔歌词行，而不是 `<br>`。

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
* **背景人声**: 使用 `<span ttm:role="x-bg" begin="..." end="...">...</span>`。背景人声的标签必须始终放在主歌词最后面。建议使用半角括号将背景人声文本包裹起来。机器人也会自动添加括号（如果没有）。**不要添加两个括号**。

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

---

### 6. 空格与格式化规范

* **空格处理**: 我们会自动规范化歌词文本中的空格，将多个连续的空格（包括换行符、制表符等）合并为一个标准的半角空格，并移除首尾的空格。**在逐字模式下，词间的空格至关重要，请务必遵循 4.1.1. 中的规则。**
* **禁止格式化**: **绝对不允许**使用任何 XML/HTML 格式化工具（如 Prettier 或 IDE 自带的格式化功能）来格式化 TTML 文件。**格式化会增加或改变 `<span>` 标签之间的独立空格文本节点，导致空格信息丢失。** 文件应保持压缩的结构，即所有字符在同一行内。
