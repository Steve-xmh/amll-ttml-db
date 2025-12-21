# TTML Lyric File Specification

### 1. Purpose and Overview

This document aims to define a standard TTML (Timed Text Markup Language) file format for the AMLL TTML Database. All lyric files submitted to this repository **must** adhere to this specification to ensure correct parsing and storage.

This document is based on the W3C TTML1 standard and has been extended for the Apple Music format.

> [!WARNING]
> For readability, the following TTML snippets are formatted. However, when uploading a TTML file, formatting is **not recommended**. See Section 6 for details.

> [!CAUTION]
> This specification is currently only effective for the experimental submission process.

---

### 2. Basic File Structure

Every TTML file must be a valid XML document and include the following basic structure and namespace declarations.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!-- The encoding must be UTF-8 and the file must not contain a Byte Order Mark (BOM). -->
<tt xmlns="http://www.w3.org/ns/ttml"
    xmlns:ttm="http://www.w3.org/ns/ttml#metadata"
    xmlns:itunes="http://itunes.apple.com/lyric-ttml-extensions"
    xmlns:amll="http://www.example.com/ns/amll"
    xml:lang="en"
    itunes:timing="Word">

    <head>
        <!-- Metadata -->
    </head>

    <body dur="00:15.500">
        <!-- Lyric contents -->
    </body>
</tt>
```

  * **XML Declaration**: The TTML file must not contain a Byte Order Mark (BOM).

  * **Root Element**: Must be `<tt>`.

  * **Namespaces**:

      * `xmlns` and `xmlns:ttm` are required by the TTML standard.
      * `xmlns:itunes` is optional, used for Apple Music-specific attributes like `itunes:timing` and `itunes:song-part`.
      * `xmlns:amll` is used for AMLL metadata.

  * **`xml:lang`**: Recommended. Specifies the primary language code of the lyrics on the `<tt>` tag (e.g., `ja` for Japanese, `en` for English).

  * **`itunes:timing`**: Optional. Used to declare line-by-line or word-by-word lyrics.
      * `word`: Word-by-word lyrics.
      * `line`: Line-by-line lyrics.
      If this attribute is not specified, the robot will automatically determine the timing mode based on the file content: if a lyric line `<p>` contains `<span>` tags with timestamps, it will be treated as **word-by-word lyrics**; otherwise, it will be treated as **line-by-line lyrics**.

  * **`body` Element**: Contains all lyric lines (`<p>`) and structural blocks (`<div>`).

      * **`dur`**: **Optional, and does not affect duration calculation. It is mainly for reference.** If included, its value **must be greater than or equal to** the end time of the last timestamp in the file. The timecodes of all inner elements must not exceed this `dur` value.

    ```xml
    <body dur="00:04:15.500">
        </body>
    ```

-----

### 3. Metadata

All metadata should be placed within the `<head><metadata>...</metadata></head>` tags.

For multiple values, a separate tag should be created for **each value**.

#### 3.1. Song and Performers

Use standard TTML tags to define basic song information and performers.

  * **Song Title**: You can use `<ttm:title>`, which will ultimately be converted to `musicName`. **Do not** add the same value in both the `<ttm:title>` and `musicName` tags.

  * **Performers**: Use `<ttm:agent>` to define all performers.

      * The `type` attribute indicates the type: `person` (solo), `group` (choir, typically use `v1000`), `other`.
      * `xml:id` provides a unique reference ID for each performer (e.g., `v1`, `v2`, `v1000`, ... are recommended).
      * The optional full name of the performer is provided within the `<ttm:name>` tag.

  * **Other `<ttm:...>` Tags**: These will eventually be converted into custom AMLL metadata tags.

      * For example, `<ttm:copyright>Some copyright info</ttm:copyright>` will be converted to `<amll:meta key="copyright" value="Some copyright info"/>`.

#### 3.2. AMLL Metadata

Use the `<amll:meta>` tag to provide core song information.

  * **Song Title**: `key="musicName"`, **Required**
  * **Artist Name**: `key="artists"`, **Required**
  * **Album Name**: `key="album"` (if it's a single, the album name should be the same as the song title), **Required**
  * **ISRC Number**: `key="isrc"`

To link the lyrics to major music platforms, **at least one** platform ID must be provided.

  * **NetEase Cloud Music**: `key="ncmMusicId"`
  * **QQ Music**: `key="qqMusicId"`
  * **Spotify**: `key="spotifyId"`
  * **Apple Music**: `key="appleMusicId"`

You can use AMLL metadata to credit the lyric author, for example:

  * **Lyric Author GitHub ID**: `key="ttmlAuthorGithub"`
  * **Lyric Author GitHub Username**: `key="ttmlAuthorGithubLogin"`

Please refer to the [AMLL Wiki](https://github.com/Steve-xmh/amll-ttml-tool/wiki/%E6%AD%8C%E8%AF%8D%E5%85%83%E6%95%B0%E6%8D%AE) **(Chinese only)** for more information.

```xml
<head>
    <metadata>
        <!-- Apple Music metadata -->
        <!-- For example only, you should not add two song title tags. -->
        <ttm:title>Song Title</ttm:title>
        <ttm:agent type="person" xml:id="v1">
            <ttm:name type="full">Artist A</ttm:name>
        </ttm:agent>
        <ttm:agent type="person" xml:id="v2">
            <ttm:name type="full">Artist B</ttm:name>
        </ttm:agent>
        <ttm:agent type="group" xml:id="v1000" />

        <!-- AMLL metadata -->
        <amll:meta key="musicName" value="Song Title" />
        <amll:meta key="musicName" value="Possible Second Song Title" />
        <amll:meta key="artists" value="Artist Name" />
        <amll:meta key="artists" value="Possible Second Artist Name" />
        <amll:meta key="album" value="Album Name"/>
        <amll:meta key="album" value="Possible Second Album Name"/>
        <amll:meta key="ncmMusicId" value="123456789"/>
        <amll:meta key="spotifyId" value="123456789"/>
        <amll:meta key="ttmlAuthorGithub" value="123456789"/>
        <amll:meta key="ttmlAuthorGithubLogin" value="Your Github Username"/>
    </metadata>
</head>
```

-----

### 4. Timing Modes

Timestamps must strictly follow these rules:

1.  **Validity**: The `begin` time must be earlier than the `end` time.

2.  **Nesting Rule**: The timestamp of a child element must be completely contained within the timestamp of its parent element.

      * Example: The time range of a `<span>` must be within the time range of its `<p>`; the time range of a `<p>` must be within the time range of its `<div>`.

3.  **Range**: All timestamps must be within the total duration of the song (the `dur` attribute of `<body>`).

4.  **Overlap**: The timestamps of `<p>` or `<span>` elements for different performers can overlap. However, the timestamps of `<p>` or `<span>` elements for the same performer cannot overlap, except in cases of time overlap due to mixing.

#### 4.1. Word-by-Word Lyrics

When `itunes:timing="Word"`:

  * Each line of lyrics is within a `<p>` tag.
  * Each syllable must be wrapped in a **`<span>` tag with `begin` and `end` attributes**.
  * For handling spaces between words, strictly follow the **4.1.1. Whitespace Handling Rules** below.

```xml
<p begin="00:01.000" end="00:03.500">
    <span begin="00:01.000" end="00:01.800">ただ一人</span>
    <span begin="00:02.000" end="00:03.500">迷い込む旅の中で</span>
</p>
```

##### 4.1.1. Whitespace Handling Rules

In word-by-word mode, spaces between words (syllables) are **significant characters** and must be explicitly represented. The robot recognizes the following methods, with the latter two being the standard.

| Method                      | Example                                             | Compliance                                 | Description                                                                                                                                                                          |
| :-------------------------- | :-------------------------------------------------- | :----------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Space inside `<span>`**   | `<span begin="00:01.0" end="00:02.0">word </span>`  | **Non-compliant(for non-formatted files)** | The robot will automatically extract leading or trailing spaces from the syllable.                                                                                                   |
| **Space outside `<span>`**  | ` <span begin="00:01.0" end="00:02.0">word</span> ` | **Most Compliant**                         | The space exists as an independent text node between two `<span>` tags.                                                                                                              |
| **Separate space `<span>`** | `<span begin="00:00.000" end="00:00.000"> </span>`  | **Allowed**                                | It is allowed to create a separate `<span>` tag for a space. It is recommended to set its `begin` and `end` time to the previous syllable's `end` time, or uniformly to `00:00.000`. |

If you submit a formatted TTML file, it is **strongly recommended** to write spaces directly inside the `span` tag to avoid strange issues. See Section 6 for details.

#### 4.2. Auto-segmentation

When timing Japanese and Korean songs, you might place multiple characters into a single timed syllable (e.g., contracted sounds). Auto-segmentation can help you evenly divide these syllables containing multiple characters, splitting the long `<span>` into multiple independent `<span>` tags, each containing a single character, and distributing the time proportionally by the number of characters.

Auto-segmentation is also applicable to lyrics obtained directly from Apple Music (which often merges multiple CJK characters with similar durations into a single syllable). You just need to add the necessary metadata and enable the auto-segmentation feature.

It is generally recommended that each `<span>` contains only **one** CJK character. Multiple CJK characters may cause an unnatural glow effect.

**Input Example:**
```xml
<span begin="10.0s" end="12.0s">你好世界</span>
```

**Output with Auto-segmentation Enabled:**
```xml
<span begin="10.000" end="10.500">你</span>
<span begin="10.500" end="11.000">好</span>
<span begin="11.000" end="11.500">世</span>
<span begin="11.500" end="12.000">界</span>
```

The auto-segmentation feature will also attempt to split an English word by its syllables, e.g., `analyse` -> `an-a-lyse`. **If this is not your desired behavior, do not enable the auto-segmentation feature.**

**Punctuation Weight** is used to control how much duration should be allocated to punctuation marks when splitting. Generally, you do not need to modify it unless you specifically want them to last shorter or longer.

#### 4.3. Line-by-Line Lyrics

When `itunes:timing="Line"`, the robot only parses the timestamp of the entire line and ignores the timestamp information of any inner `<span>` elements (which, in practice, should not exist).

*   Each line of lyrics is in a **`<p>` tag with `begin` and `end` attributes**.
*   All text content for the line is placed directly within the `<p>` tag. You can use `<span>` to add translations and other information, but the `begin` / `end` attributes of these `<span>` will be ignored.

```xml
<p begin="00:01.000" end="00:03.500">A line of lyrics</p>
```

If you are uploading line-by-line lyrics, it is recommended to enable the "**This is line-by-line lyrics**" option. Although it can be recognized without it, enabling it ensures it won't be misjudged as word-by-word lyrics.

#### 4.4 Timestamp Format

All time values in this document (e.g., the values of `begin`, `end`, `dur` attributes) **must** follow one of the formats below.

##### **Clock Time**

The recommended format is the clock-based `HH:MM:SS.fff`.

  * **`HH`**: Hours, two digits, optional.
  * **`MM`**: Minutes, two digits, required if `HH` is present.
  * **`SS`**: Seconds, two digits, required.
  * **`.fff`**: Milliseconds, optional, can be 1 to 3 digits after the decimal point.

**Omission Rules for the Format:**

  * The hours part `HH:` can be omitted, making the format `MM:SS.fff`.
  * Both the hours `HH:` and minutes `MM:` parts can be omitted, making the format `SS.fff`.
  * The milliseconds part `.fff` can be omitted.

> [!CAUTION]
> **Regarding the range of minutes and seconds**
>
>   * When a timestamp includes a colon (`:`) (i.e., in `HH:MM:SS` or `MM:SS` format), the values for minutes `MM` and seconds `SS` **must be less than 60**. For example, `01:75.000` is an **invalid** format.
>   * When a timestamp does not include a colon (e.g., `95.000`), the value for seconds can be 60 or greater.

##### **Millisecond Parsing Rules**

The parser will automatically pad the digits after the decimal point for milliseconds:

  * **1 digit**: Represents tenths of a second. For example, `15.1` will be parsed as `15` seconds and `100` milliseconds.
  * **2 digits**: Represents hundredths of a second. For example, `15.12` will be parsed as `15` seconds and `120` milliseconds.
  * **3 digits**: Represents milliseconds. For example, `15.123` will be parsed as `15` seconds and `123` milliseconds.

##### **Seconds Value**

You can directly provide a time value in seconds with an `s` suffix. This can be an integer or a floating-point number.

  * **Example**: `12.3s` represents `12300` milliseconds. `90s` represents `90000` milliseconds.

##### **Summary of Valid Formats**

| Category              | Format         | Example        | Parsed Milliseconds |
| :-------------------- | :------------- | :------------- | :------------------ |
| **Full Format**       | `HH:MM:SS.fff` | `00:02:35.500` | `155500`            |
|                       | `HH:MM:SS.f`   | `00:02:35.5`   | `155500`            |
|                       | `HH:MM:SS`     | `00:02:35`     | `155000`            |
| **Omit Hours**        | `MM:SS.ff`     | `02:35.55`     | `155550`            |
|                       | `MM:SS`        | `02:35`        | `155000`            |
| **Seconds Only**      | `SS.fff`       | `35.123`       | `35123`             |
|                       | `SS`           | `35`           | `35000`             |
|                       | `SS` (over 60) | `95`           | `95000`             |
| **`s` Suffix Format** | `f.f...s`      | `15.8s`        | `15800`             |
|                       | `fs`           | `15s`          | `15000`             |

-----

### 5. Lyric Content and Structure

#### 5.1. Lyric Components

Use `<div>` tags to segment different parts of the song (like verse, chorus) and mark them with the `itunes:song-part` attribute. This is optional content.

  * The `itunes:song-part` attribute can be set to any value, but we recommend using the following:

      * `Verse`
      * `Chorus`
      * `PreChorus`
      * `Bridge`
      * `Intro`
      * `Outro`
      * `Refrain`
      * `Instrumental`

  * A `<div>` block can have `begin` and `end` timecodes, and its time range must completely contain the times of all its child elements.

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

#### 5.2. Lines, Words, and Performers

  * **Line (`<p>`)**: The `<p>` tag is used to hold each line of the lyrics. You should use `<p>` to separate lyric lines, not `<br>`.

  * **Word (`<span>`)**: In word-by-word lyrics, `<span>` is used to mark the timing of individual words or syllables.

  * **Performer (`ttm:agent`)**: Use the `ttm:agent` attribute on the `<p>` tag, referencing the `xml:id` (e.g., `v1`) defined in the `<head>` to specify the performer.

  * **Line Number (`itunes:key`)**: Used to mark the unique number of a lyric line. The format is `L` followed by a line number starting from 1 (e.g., `L1`, `L2`, ...). The line numbers **must** be continuous and incremental, even across different `<div>` blocks.

> [!WARNING]
> Even for songs sung by a single person, you should add `ttm:agent="v1"` to the `<p>` tags and define the "v1" agent.

#### 5.3. Multi-language and Background Support

You can nest `<span>` tags within the main lyric line to provide translations, romanization, and background vocals.

> [!CAUTION]
> The AMLL suite does not yet support multiple translations or multiple romanizations. Submitting lyrics with multiple translations or romanizations is not recommended at this time.

  * **Translation**: Use `<span ttm:role="x-translation" xml:lang="language-code">...</span>`.
  * **Romanization**: Use `<span ttm:role="x-roman" xml:lang="language-Latn">...</span>`.
  *   **Background Vocals**: Use `<span ttm:role="x-bg" begin="..." end="...">...</span>`. If the background vocal appears before the main vocal, it is recommended to place the `<span ttm:role="x-bg">` tag before the main vocal's `<span>` tag. Otherwise, place the `<span ttm:role="x-bg">` tag at the end of the `<p>` tag. It is recommended to wrap the background vocal text in half-width parentheses. The robot will also automatically add parentheses (if they are missing). It is not recommended to add two or more parentheses, as although the library's robot can handle it, other parsers may not.

```xml
<p begin="00:25.100" end="00:32.500" itunes:key="L1" ttm:agent="v1">
    <!-- Main lyrics (word by word) -->
    <span begin="00:25.100" end="00:25.800">君</span>
    <span begin="00:25.900" end="00:26.100">の</span>
    <span begin="00:26.300" end="00:27.600">知らない</span>
    <span begin="00:27.700" end="00:29.100">物語</span>

    <!-- Translation content -->
    <span ttm:role="x-translation" xml:lang="zh-CN">你所不知道的物语</span>
    <span ttm:role="x-translation" xml:lang="en">The Story You Don't Know</span>

    <!-- Romaji -->
    <span ttm:role="x-roman" xml:lang="ja-Latn">kimi no shiranai monogatari</span>

    <!-- Background Vocals -->
    <span ttm:role="x-bg" begin="00:30.500" end="00:32.500">
        <!-- Main lyrics for background vocals -->
        <span begin="00:30.500" end="00:31.500">(秘密</span>
        <span begin="00:31.600" end="00:32.500">だよ)</span>

        <!-- Translation of background vocals -->
        <span ttm:role="x-translation" xml:lang="zh-CN">是秘密哦</span>
        <span ttm:role="x-translation" xml:lang="en">It's a secret</span>

        <!-- Romanization of background vocals  -->
        <span ttm:role="x-roman" xml:lang="ja-Latn">himitsu da yo</span>
    </span>
</p>
```

#### 5.4 Apple Music Style Translation

In addition to the inline auxiliary lyrics (e.g., `<span ttm:role="x-translation">...</span>`) described in `5.3`, the robot is also compatible with Apple Music style translations and transliterations defined in the `<head>`.

> [!CAUTION]
> When both formats are present, the robot will get the Apple Music style translation from the `<head>` and append it to the translation list of that line. **This may lead to double translation**.
> To avoid duplication, ensure that a translation for the same language appears in only one format. For example, if a translation with `xml:lang="zh-CN"` is defined in the `<head>`, the corresponding line in the `<body>` should not also contain a translation `<span>` with `xml:lang="zh-CN"`.

##### **Structure Explanation**

1.  **Location**: All Apple Music style auxiliary track data must be placed inside `<head><metadata>...</metadata></head>`.
2.  **Main Container**: A `<iTunesMetadata>` tag is required as a container for all Apple Music specific metadata.
3.  **Track Type Container**:
    *   **Translation**: Use the `<translations>` tag to wrap.
    *   **Transliteration**: Use the `<transliterations>` tag to wrap.
4.  **Language Block**:
    *   Inside `<translations>` or `<transliterations>`, each `<translation>` or `<transliteration>` block represents a track for one language.
    *   Each `<translation>` represents a translation in one language and **must** include the following attributes:
        *   `type="translation-type"`, which can be `subtitle` or `replacement`. `subtitle` is suitable for most translation content, while `replacement` is generally used for conversions between Simplified and Traditional Chinese.
        *   `xml:lang="language-code"` (e.g., `zh-Hans-CN`)
5.  **Text Linking**:
    *   Inside each language block, the content is carried by one or more `<text>` tags.
    *   The `for` attribute links the content to a lyric line, and its value **must** exactly match the `itunes:key` value of the corresponding `<p>` tag in the `<body>` (e.g., `for="L1"`).

##### **Content Format**

The content inside a `<text>` tag can be in one of two formats:

*   **Line-by-line**: Contains plain text for the translation or transliteration directly.
    ```xml
    <text for="L1">This is a line-by-line translation</text>
    ```

*   **Word-by-word**: Contains one or more `<span>` tags with `begin` and `end` attributes.
    ```xml
    <text for="L2">
      <span begin="10.0s" end="10.5s">A </span>
      <span begin="10.5s" end="11.0s">syllable-timed </span>
      <span begin="11.0s" end="11.8s">translation</span>
    </text>
    ```

##### **Background Vocals**

You can use `<span ttm:role="x-bg">` within a `<text>` tag to provide translations or transliterations for background vocals.

*   For **line-by-line** background vocal auxiliary lyrics, use a `<span>` with `ttm:role="x-bg"` inside the `<text>` tag.
*   For **word-by-word** background vocal auxiliary lyrics, nest timed `<span>` tags inside the `<span>` with `ttm:role="x-bg"`.

-----

##### **Example 1: Line-by-line Translation**

The following example shows how to define a Simplified Chinese translation in the header and link it to the lyric lines in the body.

**Definition in the `<head>` section:**

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

**Corresponding lyric lines in the `<body>` section:**

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

##### **Example 2: Word-by-word Translation and Transliteration**

**Lyric line in the `<body>` section:**
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

**Corresponding auxiliary tracks defined in the `<head>` section:**
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

### 6. Whitespace and Formatting Rules

#### 6.1 Formatting Support

You can use formatting tools to format your TTML file. However, any space after a `<span>` tag will be lost (if the `<span>` tag is followed by a newline).

To preserve these spaces, please write the space directly inside the `<span>` tag. For example:
```xml
<p begin="45.404" end="48.709" itunes:key="L23" ttm:agent="v1">
    <span begin="45.404" end="45.755">Gold </span>
    <span begin="45.755" end="46.696">jewelry </span>
    <span begin="46.696" end="47.627">shining </span>
    <span begin="47.627" end="47.979">so </span>
    <span begin="47.979" end="48.709">bright </span>
</p>
```

Alternatively, write two syllables that should be separated by a space on the same line:
```xml
<span begin="10s" end="11s">word1</span> <span begin="12s" end="13s">word2</span>
```

When using the experimental submission process to generate a formatted TTML file, it will, by default, write spaces directly into the `<span>` tags.

#### 6.2 Whitespace Handling

We automatically normalize whitespace in the lyric text, merging multiple consecutive spaces (including newlines, tabs, etc.) into a single standard half-width space and removing spaces inside `<span>` tags.

In **word-by-word mode**, the space between words is crucial. Although formatting is allowed, we still **strongly recommend** following the best practices defined in **4.1.1. Whitespace Handling Rules** to represent spaces between words.

-----

### 7. Language Code Specification (BCP-47)

All `xml:lang` attributes used in this document to specify a language **must** adhere to the IETF's **BCP-47** standard.

BCP-47 is the international standard for identifying human languages. It typically consists of a series of subtags separated by hyphens (`-`) to indicate language, script, region, and other information.

> [!TIP]
> You can look up all valid language codes in the [IANA Language Subtag Registry](https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry).

#### Common Examples

  * **Primary language subtag**

      * `ja`: Japanese
      * `en`: English
      * `ko`: Korean

  * **Language-Script subtag**

      * `zh-Hans`: Simplified Chinese
      * `zh-Hant`: Traditional Chinese
      * `ja-Latn`: Japanese, Romanization

  * **Language-Region subtag**

      * `en-US`: English, as used in the United States
      * `en-GB`: English, as used in the United Kingdom

  * **Language-Script-Region subtag**

      * `zh-Hans-CN`: Simplified Chinese, as used in mainland China

#### Scope of Application

This specification applies to all `xml:lang` attributes found in the file, including but not limited to:

  * **Root element**: `<tt xml:lang="...">`
  * **Inline translation**: `<span ttm:role="x-translation" xml:lang="...">`
  * **Inline romanization**: `<span ttm:role="x-roman" xml:lang="...">`
  * **Header translation**: `<translation type="subtitle" xml:lang="...">`
