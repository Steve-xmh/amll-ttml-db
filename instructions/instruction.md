# 【非必要情况下不要在一个音节内添加多个CJK文字】

# 【非必要情况下不要在一个音节内添加多个CJK文字】

# 【非必要情况下不要在一个音节内添加多个CJK文字】

###### 重要的事情先说三遍，不要问为什么！

> [!WARNING]
>
> 禁止在备注区域填写无意义或与本投稿无关的内容💢😡👊。

## 1. 元数据

> [!IMPORTANT]
>
> 请尽量使用 TTML TOOL 添加元数据，除非你已经阅读并充分理解了 TTML 文件的[具体格式规范](https://help.apple.com/itc/videoaudioassetguide/#/itc0f14fecdd)。

---

### 1.1 在 TTML TOOL 中

#### 1.1.1 填写方式

依次打开 <kbd>编辑</kbd> <kbd>编辑歌词元数据</kbd>，弹出以下界面：

![元数据编辑界面](./img/image-20250702193615365.png)

预设的元数据的获取、添加参考[本篇文章](https://github.com/Steve-xmh/amll-ttml-tool/wiki/歌词元数据)。而对于不在预设中的元数据，可以通过在 <kbd>添加新键值</kbd> 菜单中输入「自定义键名」进行添加：

![元数据添加界面](./img/image-20250702194644943.png)

#### 1.1.2 填写标准

*即便是自定义 `key` 也请尽量遵守以下标准：*

- 一个键可对应多个不同的值，但不得重复；

  > **关于一个键对应多个值**
  >
  > - 歌手/专辑/歌曲有多个名称（包括译名、别名）
  >
  >   ![艺术家官方名称 + 译名 + 别名](./img/image-20251103020146255.png)
  >
  > - 平台/官方给歌曲/专辑的标注
  >
  >   ![曲名 + 标注](./img/image-20251103020517832.png)
  >
  > - 一个组合中的所有成员及其别称或同一工作多人共同完成
  >
  >   ![组合 + 成员及译名和别称](./img/image-20251103014043436.png)
  >
  >   ![两人合奏小提琴](./img/image-20251103014814062.png)
  >
  > - 非专辑作者的共同演唱者（例如 feat/vs）
  >
  >   ![feat](./img/image-20251103020004632.png)
  >
  >   ![vs](./img/image-20251103013855676.png)
  >
  > - 一首歌对应的纯音乐版（非强制要求）
  >
  >   ![原曲 + 伴奏](./img/image-20251103013647653.png)
  >
  >   *也可以选择单独给纯音乐版本发一份 TTML，依然非强制要求*
  >
  > - 一首歌同一平台上的多个 ID 或 ISRC
  >
  >   *注：如果每个平台分开打轴，则只需要包括对应平台的 ID，简单来说——**你的这份 TTML 文件对应了该平台的哪些版本，则填写这些版本单曲的平台 ID***。
  >
  >   ![平台多个 ID + 多个 ISRC](./img/image-20251103015817511.png)
  >
  > - 同一份歌词的多名参与制作者
  >
  >   *需同时添加 `ttmlAuthorGithub` 与 `ttmlAuthorGithubLogin`*
  >
  >   ![多人共同完成一份歌词](./img/image-20251103015656931.png)
  >

- 元数据应尽量做到**准确**、**全面**；

- 元数据的值应当**只有**对应平台的 ID 或对应个人/组合的名称，请勿在**值**中填写**类型**相关内容，如果为不同类型元数据则请添加一个新的键值。

<table border="1">
<tr>
      <td>✅</td>
      <td><img alt="正确填写元数据" src="./img/image-20250702195344395.png"/></td>
</tr>
<tr>
      <td>❌</td>
      <td><img alt="错误填写元数据样例" src="./img/image-20250702195545808.png"/></td>
</tr>
</table>


以下为节选自一份带有自定义元数据歌词在 TTML TOOL 中显示的样例：

![歌词元数据样例](./img/image-20250702201505465.png)

---

### 1.2 在 TTML 文件中

> [!WARNING]
>
> 这部分不建议对于 TTML 文件格式不了解的人浏览，如果你只是一位打轴新手，或者并不关心文件代码中元数据的格式，查看上一章就足够填写标准了。

> [!CAUTION]
>
> 在 TTML 文件中，文件内容以**保留空格的** xml 标准进行压缩，文档中所给出的样例大部分为格式化之后的结果，并且可能带有注释，请在阅读时谨记。

一个元数据在 TTML 文件按照以下格式存储：

```xml
<amll:meta key="artists" value="GARNiDELiA"/>
```

#### 1.2.1 关于 `key`

其中，有一些元数据的 `key` 是已经预设好的：

- `musicName` - 歌曲名称

- `artists` - 歌曲的艺术家

- `album` - 歌曲的专辑名

- `ncmMusicId` - 网易云音乐 ID

- `qqMusicId` - QQ 音乐 ID

- `spotifyId` - Spotify 音乐 ID

- `appleMusicId` - Apple Music 音乐 ID

- `isrc` - 歌曲的 ISRC 号码

- `ttmlAuthorGithub` - 逐字作者 GitHub ID

- `ttmlAuthorGithubLogin` - 逐字歌词作者 GitHub 用户名

由于预设键将用于投稿时的解析以及审核，所以请严格按照预设填写 `key` 字段，对于非预设 `key` 则并没有额外要求。

#### 1.2.2 关于 `value`

*即便是自定义 `key` 也请尽量遵守以下标准：*

- 一个键可对应多个不同的值，但不得重复；

  > **关于一个键对应多个值**
  >
  > - 歌手/专辑/歌曲有多个名称（包括译名）
  >
  >   ```xml
  >   <amll:meta key="artists" value="滨崎步" />
  >   <amll:meta key="artists" value="はまさき あゆみ" />
  >   <amll:meta key="artists" value="浜崎あゆみ" />
  >   <amll:meta key="artists" value="浜﨑あゆみ" />
  >   <amll:meta key="artists" value="浜崎くるみ" />
  >   <amll:meta key="artists" value="Ayumi Hamasaki" />
  >   <amll:meta key="artists" value="AYUMI" />
  >   <amll:meta key="artists" value="CREA" />
  >   ```
  >
  > - 平台/官方给歌曲/专辑的标注
  >
  >   ```xml
  >   <amll:meta key="musicName" value="極楽浄土" />
  >   <amll:meta key="musicName" value="极乐净土" />
  >   <amll:meta key="musicName" value="Gokuraku Joudo" />
  >   <amll:meta key="musicName" value="游戏《阴阳师》宣传曲" />
  >   <amll:meta key="musicName" value="NetEaseアプリゲーム「陰陽師」キャンペーン用テーマソング" />
  >   ```
  >
  > - 一个组合中的所有成员及其别称或同一工作多人共同完成
  >
  >   ```xml
  >   <amll:meta key="artists" value="NIJISANJI EN" />
  >   <amll:meta key="artists" value="Ren Zotto" />
  >   <amll:meta key="artists" value="レン ゾット" />
  >   <amll:meta key="artists" value="れん ぞっと" />
  >   <amll:meta key="artists" value="Doppio Dropscythe" />
  >   <amll:meta key="artists" value="ドッピオ ドロップサイト" />
  >   <amll:meta key="artists" value="どっぴお どろっぷさいと" />
  >   <amll:meta key="artists" value="豆皮" />
  >   ```
  >
  >   ```xml
  >   <amll:meta key="Violin" value="日俣綾子" />
  >   <amll:meta key="Violin" value="土屋玲子" />
  >   ```
  >
  > - 非专辑作者的共同演唱者（例如 feat/vs）
  >
  >   ```xml
  >   <!-- Just Be Friends (feat. 小豆沢こはね) - Vivid BAD SQUAD -->
  >   <amll:meta key="artists" value="Vivid BAD SQUAD" />
  >   <amll:meta key="artists" value="炫狂小队" />
  >   <amll:meta key="artists" value="小豆沢心羽" />
  >   <amll:meta key="artists" value="小豆沢こはね" />
  >   <amll:meta key="artists" value="Azusawa Kohane" />
  >   ```
  >
  >   ```xml
  >   <!-- Poppin'Trip -GARNiDELiA vs HEAVYGRINDER- - GARNiDELiA -->
  >   <amll:meta key="artists" value="GARNiDELiA" />
  >   <amll:meta key="artists" value="ガルニデリア" />
  >   <amll:meta key="artists" value="加尼德里亚" />
  >   <amll:meta key="artists" value="メイリア" />
  >   <amll:meta key="artists" value="MARiA" />
  >   <amll:meta key="artists" value="水桥舞" />
  >   <amll:meta key="artists" value="美依礼芽" />
  >   <amll:meta key="artists" value="とくP" />
  >   <amll:meta key="artists" value="阿部尚徳" />
  >   <amll:meta key="artists" value="TOKU" />
  >   <amll:meta key="artists" value="toku" />
  >   <amll:meta key="artists" value="HEAVYGRINDER" />
  >   ```
  >
  > - 一首歌对应的纯音乐版（非强制要求）
  >
  >   ```xml
  >   <amll:meta key="musicName" value="Error" />
  >   <amll:meta key="musicName" value="TV动画《BEATLESS》片头曲" />
  >   <amll:meta key="musicName" value="Error instrumental" />
  >   <amll:meta key="musicName" value="Error (Instrumental)" />
  >   <amll:meta key="musicName" value="Error (Inst.)" />
  >   ```
  >
  >   *也可以选择单独给纯音乐版本发一份 TTML，依然非强制要求*
  >
  > - 一首歌同一平台上的多个 ID 或 ISRC
  >   *注：如果每个平台分开打轴，则只需要包括对应平台的 ID，简单来说——**你的这份 TTML 文件对应了该平台的哪些版本的单曲，则填写这些单曲的平台 ID**。*
  >
  >   ```xml
  >   <amll:meta key="ncmMusicId" value="2666660478" />
  >   <amll:meta key="qqMusicId" value="557082683" />
  >   <amll:meta key="spotifyId" value="5WGoYp5w3ZRnlD8XGctrJq" />
  >   <amll:meta key="spotifyId" value="0Ggb40fWT3QQrPozImxM1P" />
  >   <amll:meta key="spotifyId" value="1StslCM3f5t1arKcjWjnZ3" />
  >   <amll:meta key="spotifyId" value="41dRorJGXRTHjFnTGApQeV" />
  >   <amll:meta key="appleMusicId" value="1791675586" />
  >   <amll:meta key="appleMusicId" value="1791844169" />
  >   <amll:meta key="appleMusicId" value="1791845725" />
  >   <amll:meta key="appleMusicId" value="1791844189" />
  >   <amll:meta key="isrc" value="FR10S2554585" />
  >   <amll:meta key="isrc" value="FR10S2564135" />
  >   <amll:meta key="isrc" value="FR10S2564142" />
  >   <amll:meta key="isrc" value="FR10S2564161" />
  >   ```
  >
  > - 同一份歌词的多名参与制作者
  >
  >   *需同时添加 `ttmlAuthorGithub` 与 `ttmlAuthorGithubLogin`*
  >
  >   ```xml
  >   <amll:meta key="ttmlAuthorGithub" value="68000793" />
  >   <amll:meta key="ttmlAuthorGithubLogin" value="ranhengzhang" />
  >   <amll:meta key="ttmlAuthorGithub" value="50987405" />
  >   <amll:meta key="ttmlAuthorGithubLogin" value="ITManCHINA" />
  >   ```

- 元数据应尽量做到**准确**、**全面**；

- 元数据的值应当**只有**对应平台的 ID 或对应个人/组合的名称，请勿在**值**中填写**类型**相关内容，如果为不同类型元数据则请添加一个新的键值。

<table border="1">
  <tr>
    <td>✅</td>
    <td><code>&lt;amll:meta key="biliId" value="BV1QL4y1L7CP"/&gt;</code></td>
  </tr>
  <tr>
    <td>✅</td>
    <td><code>&lt;amll:meta key="演唱" value="メイリア"/&gt;</code></td>
  </tr>
  <tr>
    <td>❌</td>
    <td><code>&lt;amll:meta key="vocal" value="演唱：メイリア"/&gt;</code></td>
  </tr>
</table>

## 2. 歌词正文

- 不得出现违反法律法规的内容；

- 不得包含歌词内容以外的信息，例如作词、作曲等信息应当放在元数据部分；

> [!WARNING]
>
> 如表演者为团体，且需要注明每一句的具体演唱者时，可在翻译行的开头注明该句的演唱者；

- 不得包含 Emoji 或颜文字等无关内容；

  - 版权方提供的原始歌词文本中包含这些内容时除外；

- 内容真实、准确；

> [!WARNING]
>
> 出自游戏、电影等其他作品中的专有名词须进行校对以保证准确性；

- 易混淆或易拼写错误的专有名词须注明出处以便考证；

- 歌词行的结尾不得使用句号或逗号，但可以按需使用感叹号、问号和引号； [[来源]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea:~:text=歌词行的结尾不得使用句号或逗号，但可以按需使用感叹号、问号和引号。)

- 每行歌词都应遵循惯用的语法规则。 [[来源]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea:~:text=和引号。-,每行歌词都应遵循惯用的语法规则。,-其他可在歌词)

---

### 2.1 英语限定

- 请遵循惯用的英文语法规则。专有名词必须首字母大写。此外，每行歌词中第一个单词必须首字母大写。

> [!TIP]
>
> 有可信来源证明其原作者使用非规范形式大/小写文本的除外。

- 其他需要首字母大写的情况：

  - 与神和宗教相关的单词（宗教歌曲中的 God、You、Him、Your 等）；

  - 缩略词大小写请遵循使用习惯（NASA、FBI 等）；

  - 地理位置（East Coast、Southside 等）；

  - 作品名称；

  - 括号中第一个单词的首字母；

  - 品牌名称。

[[来源]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea)

---

### 2.2 日语限定

- 尽可能在正文中使用方括号（「」『』）作为其他语言中的引号和单引号的代替；

- 不要把汉字对应的假名在文本中用括号列出来（包括<ruby>義訓<rt>ギクン</rt></ruby>）：

<table border="1">
  <tr>
    <td>✅</td>
    <td>あの時こう……</td>
  </tr>
  <tr>
    <td>✅</td>
    <td>あのときこう……</td>
  </tr>
  <tr>
    <td>❌</td>
    <td>あの時（とき）こう…… </td>
  </tr>
</table>

## 3. 翻译和音译

### 3.1 翻译

> [!WARNING]
>
> - 涉及政治敏感、有违人道主义的曲目不得提供歌词翻译；
>
>   - ❌ 如果为上述情况的国语歌曲则**不予提交歌词**。
>
> - 对于有可能 NSFW 的翻译，可以忠于原文，但请尽量注意尺度，否则可能因尺度过大不予通过；
>
> - 为避免可能的版权纠纷，使用 B站等视频平台中作品包含的原创翻译内容时，请标注来源；

- 不得出现违反法律法规及公序良俗的内容；

- 对于非国语歌词，请尽可能提供真实、准确的翻译和音译；

- 原则上禁止在翻译中插入 Emoji 或颜文字等无关内容，除非翻译作品的版权方也是这么做的；

- 原则上不建议在翻译行末尾保留逗号或句号；

### 3.2 音译

- 对于注音部分则不应当出现**全角**标点符号；

- 注音的内容并没有统一标准，但是一份歌词中**使用相同语言代码的一组音译**应当符合同一份标准。

  > 例如：一份日语歌词中的注音可以是遵循黑本式/训令式/护照式的，并且标记 `xml:lang="ja-Latn"`，但是在同一份歌词中**不能同时**将这几种标准的音译都放在 `xml:lang="ja-Latn"` 标记的音译中，如果需要上传多种标准的音译，则需要使用不同语言代码进行区分：`ja-Latn-hepburn` `ja-Latn-kunrei` `ja-Latn-passport`，如果 BCP-47 标准中没有则可以使用 `x-` 前缀**私有扩展**：`ja-Latn-x-hepburn` `ja-Latn-x-kunrei` `ja-Latn-x-passport`。

- 并不强制要求使用空格严格分隔，但是不管是否使用空格分隔，注音都需要符合以下标准：**内容准确**和**无二义性**。以日语为例：

  - 音译应当尽量使用空格分隔每个字的音译，尽量分隔每个**假名**的音译：

    > <b><ruby>思<rt>おも</rt></ruby></b>う/**o mo** u

  - 日语中的「ん」的音译 "n/m" 之前可以不用空格分隔；

  - 对于日语的音译如果以**汉字/单词**为单位分隔，并且「ん」后紧跟 a 行，则请在「ん」的注音之后添加一个半角单引号避免二义性：

    > <ruby>会議<rt>かいぎ</rt></ruby>で<ruby>彼<rt>かれ</rt></ruby>の<ruby>真意<rt>しんい</rt></ruby>を<ruby>理解<rt>りかい</rt></ruby>した/kaigi de kare no **shin'i** o rikai shita

### 3.3 逐字音译/翻译

> [!IMPORTANT]
>
> 提交之前请保证你的带有逐字音译/翻译的歌词文件能够在 AMLL Player 和 AM 网页端之中至少一方能够正常显示。

> [!CAUTION]
>
> 逐字翻译是简体中文和繁体中文在显示时进行互换的一种方式，目前除了 AM 之外没有软件实现这种功能，在 AMLL Player 中也只会显示为逐行翻译。

#### 3.3.1 在 TTML TOOL 中

- 音译中的每个注音之间应当尽量使用空格进行分割：

  > <kbd>圆</kbd> <kbd>是</kbd> <kbd>美</kbd> <kbd>满</kbd>
  >
  > <kbd>jyun4·</kbd> <kbd>si6·</kbd> <kbd>mei5·</kbd> <kbd>mun5</kbd>

- 标点的音译要么使用对应的半角符号，要么不填写：

  > <kbd>「</kbd> <kbd><ruby>未<rt>願</rt></ruby></kbd> <kbd><ruby>来<rt>い</rt></ruby></kbd> <kbd>」</kbd> <kbd>「</kbd> <kbd><ruby>希<rt>願</rt></ruby></kbd> <kbd><ruby>望<rt>い</rt></ruby></kbd> <kbd>」</kbd>
  >
  > <kbd>“</kbd> <kbd>ne·ga·</kbd> <kbd>i</kbd> <kbd>”</kbd> <kbd>“</kbd> <kbd>ne·ga·</kbd> <kbd>i</kbd> <kbd>”</kbd>

- 日语中音译连续但是被分开为多个音节（「ん」和促音）时，之间不需要使用空格分隔：

  > <kbd><ruby>心<rt>こころ</rt></ruby></kbd> <kbd>の</kbd> <kbd><ruby>準<rt>じゅん</rt></ruby></kbd> <kbd><ruby>備<rt>び</rt></ruby></kbd> <kbd>な</kbd> <kbd>ん</kbd> <kbd>て</kbd> <kbd><ruby>言<rt>い</rt></ruby></kbd> <kbd>っ</kbd> <kbd>て</kbd> <kbd>る</kbd> <kbd><ruby>暇<rt>ひま</rt></ruby></kbd> <kbd>は</kbd> <kbd>ない</kbd>
  >
  > <kbd>ko·ko·ro·</kbd> <kbd>no·</kbd> <kbd>jum·</kbd> <kbd>bi·</kbd> <b><mark><kbd>na</kbd> <kbd>n·</kbd></mark></b> <kbd>te·</kbd> <kbd>i·</kbd> <b><mark><kbd>t</kbd> <kbd>te·</kbd></mark></b> <kbd>ru·</kbd> <kbd>hi·ma·</kbd> <kbd>wa·</kbd> <kbd>na·i</kbd>

#### 3.3.2 在 TTML 文件中

> [!CAUTION]
>
> 以下小节中的 TTML 代码来源于规范压缩格式，**仅进行了换行处理**。所有行首空格均为原始内容的一部分，不代表语法缩进。

- 对于歌词文件、翻译、音译的语言标记，应当使用遵循 IETF 的 BCP-47 标准的正确语言类型标记，目前常用语言类型有以下几种：

  - `zh-Hans` - 简体中文
  - `zh-Hant` - 繁体中文 (粤语同样使用)
  - `zh-Latn-pinyin` - 中文拼音 (不分繁简)
  - `zh-Latn-jyutping` - 粤语注音
  - `en` - 英文
  - `ja` - 日语
  - `ja-Latn` - 日语罗马音
  - `ko` - 韩语
  - `ko-Latn` - 韩语罗马音

- 音译中的每个注音之间应当尽量使用**纯文本节点形式**的空格进行分隔：

  > 圓是美滿/jyun4<mark> </mark>si6<mark> </mark>mei5<mark> </mark>mun5
  >
  > ```xml
  > <p begin="16.220" end="21.620" itunes:key="L1" ttm:agent="v1">
  > <span begin="16.220" end="16.440">圓</span>
  > <span begin="16.440" end="16.780">是</span>
  > <span begin="16.780" end="17.150">美</span>
  > <span begin="17.150" end="17.710">滿</span>
  > </p>
  > ```
  >
  > <pre lang="xml">&lt;text for=&quot;L1&quot;&gt;
  > &lt;span begin=&quot;16.220&quot; end=&quot;16.440&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;jyun4&lt;/span&gt;
  > <mark> </mark>&lt;span begin=&quot;16.440&quot; end=&quot;16.780&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;si6&lt;/span&gt;
  > <mark> </mark>&lt;span begin=&quot;16.780&quot; end=&quot;17.150&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;mei5&lt;/span&gt;
  > <mark> </mark>&lt;span begin=&quot;17.150&quot; end=&quot;17.710&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;mun5&lt;/span&gt;
  > &lt;/text&gt;</pre>

- 标点的音译要么使用对应的半角符号，要么不填写；

  > 「<ruby>未来<rt>願い</rt></ruby>」「<ruby>希望<rt>願い</rt></ruby>」/==“==ne ga i==”“==ne ga i==”==
  >
  > <pre>&lt;p begin=&quot;3:52.868&quot; end=&quot;3:56.138&quot; itunes:key=&quot;L52&quot; ttm:agent=&quot;v1&quot;&gt;
  > &lt;span begin=&quot;3:52.863&quot; end=&quot;3:52.868&quot;&gt;<b><mark>「</mark></b>&lt;/span&gt;
  > &lt;span begin=&quot;3:52.868&quot; end=&quot;3:53.298&quot;&gt;未&lt;/span&gt;
  > &lt;span begin=&quot;3:53.358&quot; end=&quot;3:54.198&quot;&gt;来&lt;/span&gt;
  > &lt;span begin=&quot;3:54.198&quot; end=&quot;3:54.203&quot;&gt;<b><mark>」</mark></b>&lt;/span&gt;
  > &lt;span begin=&quot;3:54.728&quot; end=&quot;3:54.733&quot;&gt;<b><mark>「</mark></b>&lt;/span&gt;
  > &lt;span begin=&quot;3:54.728&quot; end=&quot;3:55.148&quot;&gt;希&lt;/span&gt;
  > &lt;span begin=&quot;3:55.228&quot; end=&quot;3:56.138&quot;&gt;望&lt;/span&gt;
  > &lt;span begin=&quot;3:56.138&quot; end=&quot;3:56.143&quot;&gt;<b><mark>」</mark></b>&lt;/span&gt;
  > &lt;/p&gt;</pre>
  >
  > <pre>&lt;text for=&quot;L52&quot;&gt;
  > &lt;span begin=&quot;3:52.863&quot; end=&quot;3:52.868&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;<b><mark>“</mark></b>&lt;/span&gt;
  > &lt;span begin=&quot;3:52.868&quot; end=&quot;3:53.298&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;ne&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;3:53.358&quot; end=&quot;3:54.198&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;ga i&lt;/span&gt;
  > &lt;span begin=&quot;3:54.198&quot; end=&quot;3:54.203&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;<b><mark>”</mark></b>&lt;/span&gt;
  > &lt;span begin=&quot;3:54.728&quot; end=&quot;3:54.733&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;<b><mark>“</mark></b>&lt;/span&gt;
  > &lt;span begin=&quot;3:54.728&quot; end=&quot;3:55.148&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;ne&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;3:55.228&quot; end=&quot;3:56.138&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;ga i&lt;/span&gt;
  > &lt;span begin=&quot;3:56.138&quot; end=&quot;3:56.143&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;<b><mark>”</mark></b>&lt;/span&gt;
  > &lt;/text&gt;</pre>

- 日语中音译连续但是被分开为多个音节（「ん」和促音）时，不需要添加空格：

  > 心の準備==なん==て言==って==る暇はない/ko ko ro no ==jum== bi nan te i ==tte== ru hi ma wa na i
  >
  > <pre lang="xml">&lt;p begin=&quot;2:20.288&quot; end=&quot;2:23.488&quot; itunes:key=&quot;L27&quot; ttm:agent=&quot;v1&quot;&gt;
  > &lt;span begin=&quot;2:20.288&quot; end=&quot;2:20.678&quot;&gt;心&lt;/span&gt;
  > &lt;span begin=&quot;2:20.678&quot; end=&quot;2:20.848&quot;&gt;の&lt;/span&gt;
  > &lt;span begin=&quot;2:20.848&quot; end=&quot;2:21.078&quot;&gt;準&lt;/span&gt;
  > &lt;span begin=&quot;2:21.078&quot; end=&quot;2:21.148&quot;&gt;備&lt;/span&gt;
  > <b><mark>&lt;span begin=&quot;2:21.148&quot; end=&quot;2:21.308&quot;&gt;な&lt;/span&gt;
  > &lt;span begin=&quot;2:21.308&quot; end=&quot;2:21.458&quot;&gt;ん&lt;/span&gt;</mark></b>
  > &lt;span begin=&quot;2:21.458&quot; end=&quot;2:21.668&quot;&gt;て&lt;/span&gt;
  > &lt;span begin=&quot;2:21.668&quot; end=&quot;2:21.818&quot;&gt;言&lt;/span&gt;
  > <b><mark>&lt;span begin=&quot;2:21.818&quot; end=&quot;2:21.898&quot;&gt;っ&lt;/span&gt;
  > &lt;span begin=&quot;2:21.898&quot; end=&quot;2:22.088&quot;&gt;て&lt;/span&gt;</mark></b>
  > &lt;span begin=&quot;2:22.088&quot; end=&quot;2:22.228&quot;&gt;る&lt;/span&gt;
  > &lt;span begin=&quot;2:22.228&quot; end=&quot;2:22.728&quot;&gt;暇&lt;/span&gt;
  > &lt;span begin=&quot;2:22.728&quot; end=&quot;2:22.838&quot;&gt;は&lt;/span&gt;
  > &lt;span begin=&quot;2:22.838&quot; end=&quot;2:23.488&quot;&gt;ない&lt;/span&gt;
  > &lt;/p&gt;</pre>
  >
  > <pre lang="xml">&lt;text for=&quot;L27&quot;&gt;
  > &lt;span begin=&quot;2:20.288&quot; end=&quot;2:20.678&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;ko ko ro&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;2:20.678&quot; end=&quot;2:20.848&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;no&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;2:20.848&quot; end=&quot;2:21.078&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;jum&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;2:21.078&quot; end=&quot;2:21.148&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;bi&lt;/span&gt;
  > <span> </span><b><mark>&lt;span begin=&quot;2:21.148&quot; end=&quot;2:21.308&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;na&lt;/span&gt;
  > &lt;span begin=&quot;2:21.308&quot; end=&quot;2:21.458&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;n&lt;/span&gt;
  > </mark></b> &lt;span begin=&quot;2:21.458&quot; end=&quot;2:21.668&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;te&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;2:21.668&quot; end=&quot;2:21.818&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;i&lt;/span&gt;
  > <span> </span><b><mark>&lt;span begin=&quot;2:21.818&quot; end=&quot;2:21.898&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;t&lt;/span&gt;
  > &lt;span begin=&quot;2:21.898&quot; end=&quot;2:22.088&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;te&lt;/span&gt;</mark></b>
  > <span> </span>&lt;span begin=&quot;2:22.088&quot; end=&quot;2:22.228&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;ru&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;2:22.228&quot; end=&quot;2:22.728&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;hi ma&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;2:22.728&quot; end=&quot;2:22.838&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;wa&lt;/span&gt;
  > <span> </span>&lt;span begin=&quot;2:22.838&quot; end=&quot;2:23.488&quot; xmlns=&quot;http://www.w3.org/ns/ttml&quot;&gt;na i&lt;/span&gt;
  > &lt;/text&gt;</pre>

> [!TIP]
>
> 可以在 AMLL Player 中查看**行**音译部分的内容，依此检查逐字音译的空格设置是否有误：
>
> ![根据行音译内容检查逐字音译的空格](./img/image-20260113020619009.png)

## 4. 排版

> [!WARNING]
>
> 文本行的长度应符合通常的书面表达习惯。  
> 不应过度将一行文本拆得特别短，文本长度至少应完整保留到语义上的“逗号”（半句）；如果长度允许，应整句保留。
>
> 但以下情况除外：
> - 这一整句由不同的歌手轮流或交替演唱，你需要为每一位歌手创建单独的一行；
> - 使用常见 PC 显示器（1080P）、缩放设置为 100% 、标准字体大小、最大化窗口（或全屏模式）下，这行歌词的长度仍然超过了排版系统的限制，导致出现了不合理的换行；
>
> 例如：
> <table border="1" style="text-align: center;">
>    <thead>
>      <tr>
>        <th>原文</th>
>        <th>拆分后的文本</th>
>        <th></th>
>      </tr>
>    </thead>
>    <tbody>
>      <tr style="vertical-align: middle;"><td>我要这山断不了来路 我要这水挡不住归途 我要天地都为我让步</td><td>我要这山断不了来路<br>我要这水挡不住归途<br>我要天地都为我让步</td><td>✅</td></tr>
>      <tr style="vertical-align: middle;"><td>剑出鞘 恩怨了 谁笑</td><td>剑出鞘<br>恩怨了<br>谁笑</td><td>❌</td></tr>
>      <tr style="vertical-align: middle;"><td><ruby>期待されないまま<rt>流歌×咲希&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</rt></ruby> <ruby>期待もしないまま<rt>流歌×穗波&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</rt></ruby></td>
>           <td><ruby>期待されないまま<rt>流歌×咲希&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</rt></ruby><br><ruby>期待もしないまま<rt>流歌×穗波&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</rt></ruby></td><td>✅<br>但也可以不分行</td></tr>
>    </tbody>
> </table>

- 尽可能按照单个文字或单个音节制作逐字/逐音节歌词；

- 在机器人处理后的文件中，不允许出现空格包含在音节内**首尾**的情况；

- 不允许包含空白行；

- 按照各语言的书写习惯合理选择使用半角或全角标点符号；

- 在不影响时间轴精确度的情况下，可以将标点符号作为单独的单词打轴；

- 合理使用对唱/背景视图：

  - 背景人声的歌词应单独为一行，放在主唱人声歌词的下一行并标记为背景行；

    - （非强制要求）不应因背景行而延长主行时间轴的持续时间。

  - 如果不确定何时换行或分段，请参考以下划分依据： [[来源 1]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea)[[来源 2]](https://help.apple.com/itc/musicstyleguide/#/itc3ae5d4dea)

    - Apple Music 所建议的组成部分：

      > - Verse（主歌）
      >
      > - Chorus（副歌）
      >
      > - PreChorus（预副歌）
      >
      > - Bridge（桥段）
      >
      > - Intro（前奏）
      >
      > - Outro（尾奏）
      >
      > - Refrain（叠句）
      >
      > - Instrumental（器乐）
      >
      > - Hook（钩子）

    - 歌曲节拍速度发生变化；

    - 艺人对歌词的演绎方式发生变化（例如，从歌唱切换成说唱）。

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
> <span ttm:role="x-bg" begin="02:23.620" end="02:25.690">
> 	<span begin="02:23.620" end="02:24.100">(In</span>
> 	<!-- 此处空格被格式化 -->
> 	<span begin="02:24.100" end="02:24.380">your</span>
> 	<!-- 此处空格被格式化 -->
> 	<span begin="02:24.380" end="02:25.690">heart)</span>
> </span>
> ```
>
> 但如果在 TTML TOOL 中手动再添加一次括号：
>
> > <kbd>(In</kbd> <kbd>空格x1</kbd> <kbd>your</kbd> <kbd>空格x1</kbd> <kbd>heart)</kbd>
>
> 那么导出时就会变为：
>
> ```xml
> <span ttm:role="x-bg" begin="02:23.620" end="02:25.690">
> 	<span begin="02:23.620" end="02:24.100">((In</span>
> 	<!-- 此处空格被格式化 -->
> 	<span begin="02:24.100" end="02:24.380">your</span>
> 	<!-- 此处空格被格式化 -->
> 	<span begin="02:24.380" end="02:25.690">heart))</span>
> </span>
> ```
>
> 成为一种不合规的形式，将无法通过审核。

---

### 4.1 英语限定

在 PR 提交的文件（包括机器人处理后的文件）中：

- 音节与音节之间的空格独立成音节；

- 不得将空格包含在单词首尾；

- 不允许使用一个以上的空格来分隔单词。

<table border="1">
  <tr>
    <td>❌</td>
    <td><code>This \ is\  a  \lyric</code><br/><kbd>This·</kbd><kbd>·is</kbd><kbd>··a··</kbd><kbd>lyric</kbd></td>
  </tr>
  <tr>
    <td>❌</td>
    <td><code>This \is \a \lyric</code><br/><kbd>This·</kbd><kbd>is·</kbd><kbd>a·</kbd><kbd>lyric</kbd></td>
  </tr>
  <tr>
    <td>❌</td>
    <td><code>This\ is\ a\ lyric</code><br/><kbd>This</kbd><kbd>·is</kbd><kbd>·a</kbd><kbd>·lyric</kbd></td>
  </tr>
  <tr>
    <td>✅</td>
    <td><code>This\ \is\ \a\ \lyric</code><br/><kbd>This</kbd><kbd>空格x1</kbd><kbd>is</kbd><kbd>空格x1</kbd><kbd>a</kbd><kbd>空格x1</kbd><kbd>lyric</kbd></td>
  </tr>
  <tr>
    <td>✅</td>
    <td><code>Thi\s i\s a\ \ly\ri\c</code><br/><kbd>Thi</kbd><kbd>s·i</kbd><kbd>s·a</kbd><kbd>空格x1</kbd><kbd>ly</kbd><kbd>ri</kbd><kbd>c</kbd></td>
  </tr>
</table>


## 5. HOYO-MiX

- 在以上要求的基础之上，要求将尽可能多地区的元数据添加到文件中；
  - 当前，Apple 和 Spotify 分为下面这些分区，请按照这些分区逐一查找并添加元数据：[参考](https://github.com/Steve-xmh/amll-ttml-db/issues/3366)
    - 简中区（Spotify 无简中区）；
    - 繁中区；
    - 韩语区；
    - 日语区；
    - 其它区（除了以上地区以外的其它地区，语言为英语）。
- 必须是逐字歌词（如果是英文则应当精细到音节）；
- 必须适配艺人演唱时的各类效果。

> **关于查找不同区的元数据**
>
> Apple Music 可以通过 AME 插件切换分区查找：
>
> ![AME 插件查找歌曲信息](./img/image-20250821184933745.png)
>
> - 简中区：Chinese Mainland
>
> - 繁中区：Hong Kong SAR, Macao SAR, Taiwan Province, People's Republic of China
>
> - 韩语区：Korea, Republic of
>
> - 日语区：Japan
>
> - 其它区：United States、Russia、France、Vietnam、Brazil, etc.
>
> 但是这种方式只适用于**同一专辑在各个分区有着不同信息**的情况，而 HOYO-MiX 更倾向于在每个区发一份独占专辑，因此就需要访问各区的链接进行搜索：
>
> - 简中区：https://beta.music.apple.com/cn/search
>
> - 繁中区（三选一即可）：
>
>   - https://beta.music.apple.com/hk/search
>
>   - https://beta.music.apple.com/mo/search
>
>   - https://beta.music.apple.com/tw/search
>
> - 韩语区：https://beta.music.apple.com/kr/search
>
> - 日语区：https://beta.music.apple.com/jp/search
>
> - 其它区：https://beta.music.apple.com/us/search
>
> Spotify 可以使用 Lyricify 4 的搜索工具进行换区搜索（可能需要更换对应语言的歌曲名）：
>
> ![使用 Lyricify4 查找歌曲信息](./img/image-20250728160546661.png)

## 6. 其他

- 请善用结束时间来让歌词播放器自动生成间奏区域，不要为了演出效果强制改变为错误的时间轴，例如为了不触发间奏强行延后上一句结束时间和提早下一句开始时间或为了触发间奏强行提前上一行结束时间和延后下一句开始时间；

- 歌曲作者等信息请使用 AMLL TTML Tool 的元数据功能添加（包括但不限于预设的字段）；

- 提交时标题中的「歌词提交/修正」尽量根据实际提交内容改为「歌词修正」或「歌词提交」；

- 如果是对已有歌词的修正，请在补充说明处写明修改原因提供给审核核对，否则将被退回。
