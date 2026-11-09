> [!NOTE]
> **这是 [cardisoft/cardinal](https://github.com/cardisoft/cardinal) 的个人衍生版（CardinalM）。**
> 以下为上游原文。本衍生版在上游 master 基础上增加了：
>
> - **按名称忽略规则**——「偏好设置」中新增「忽略规则」多行输入框（位于「忽略路径」下方），每行一条 .gitignore 风格规则：`*.log`、`.DS_Store`、`~$*` 用于文件；规则以 `/` 结尾表示只匹配目录，例如 `.*/` 剪掉所有点开头的目录，`node_modules/` 剪掉任意层级的依赖目录。命中的文件或目录子树不会进入「文件名索引」和「内容索引」，而不只是从搜索结果中隐藏。文件规则绝不匹配目录，目录规则也绝不匹配文件；匹配不区分大小写（ASCII）。
> - **关窗即退出**——关闭主窗口时退出应用，而不是像上游那样隐藏到后台。
>
> 实现说明：匹配器是独立、零依赖的 Rust 模块（`fswalk/src/ignore_file.rs`）；规则以指纹形式写入缓存，因此只有规则真正变化才会触发重建；缓存格式版本号与上游错开以免冲突。本衍生版长期跟随上游 rebase，改动尽量小而集中。
>
> **下载编译好的版本（Apple Silicon）**：[Releases 页面](https://github.com/kingdocs/cardinalm/releases)

<div align="center">
  <img src="../../cardinal/mac-icon_1024x1024.png" alt="Cardinal icon" width="120" height="120">
  <h1>Cardinal</h1>
  <p>最快最准的 macOS 文件搜索应用</p>
  <p>
    <a href="#使用-cardinal">使用 Cardinal</a> ·
    <a href="#构建-cardinal">构建 Cardinal</a>
  </p>
  <img src="UI.gif" alt="Cardinal UI preview" width="720">
</div>

---

[English](../../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## 使用 Cardinal

### 下载

通过 Homebrew 安装：

```bash
brew install --cask cardinal-search
```

你也可以从 [GitHub Releases](https://github.com/cardisoft/cardinal/releases/) 获取最新的安装包。

### 国际化支持

想切换其他语言？点击状态栏里的 ⚙️ 按钮即可即时切换。

### 基础搜索语法

Cardinal 现在在经典的子串/前缀匹配基础上叠加了 Everything 兼容语法：

- `report draft` – 空格代表 `AND`，只会得到同时包含两个词的文件。
- `*.pdf briefing` – 只显示文件名包含 “briefing” 的 PDF 结果。
- `*.zip size:>100MB` – 查找大于 100MB 的 ZIP 文件。
- `in:/Users demo !.psd` – 把搜索范围限制在 `/Users`，然后匹配包含 `demo` 但排除 `.psd` 的文件。
- `tag:ProjectA;ProjectB` – 按 Finder 标签（macOS）过滤；`;` 表示 `OR`（满足任一标签即可）。
- `*.md content:"Bearer "` – 仅筛选包含字符串 `Bearer ` 的 Markdown 文件。
- `"Application Support"` – 使用引号匹配完整短语。
- `brary/Applicat` – 使用 `/` 作为路径分隔符向下匹配子路径，例如 `Library/Application Support`。
- `/report` · `draft/` · `/report/` – 在词首/词尾添加 `/`，分别强制匹配前缀、后缀或精确文件名，补足 Everything 语法之外的整词控制。
- `~/**/.DS_Store` – `**` 会深入所有子目录，在整个家目录中查找散落的 `.DS_Store` 文件。

更多支持的操作符（布尔组合、文件夹限定、扩展名过滤、正则示例等）请参见 [`search-syntax.zh-CN.md`](search-syntax.zh-CN.md)。

### 键盘快捷键与预览

- `Cmd+Shift+Space` – 通过全局快捷键开/关 Cardinal 窗口。
- `Cmd+,` – 打开偏好设置。
- `Esc` – 隐藏 Cardinal 窗口。
- `ArrowUp`/`ArrowDown` – 上下移动选中项。
- `Shift+ArrowUp`/`Shift+ArrowDown` – 扩展选中范围。
- `Space` – 不离开 Cardinal 即可对当前行执行 Quick Look。
- `Cmd+O` – 打开选中的结果。
- `Cmd+R` – 在 Finder 中定位选中的结果。
- `Cmd+C` – 复制所选文件到剪贴板。
- `Cmd+Shift+C` – 复制所选路径到剪贴板。
- `Cmd+F` – 焦点回到搜索框。
- `ArrowUp`/`ArrowDown`（在搜索框内）– 浏览搜索历史。

祝你搜索愉快！

---

## 构建 Cardinal

### 环境要求

- macOS 12+
- Rust 工具链
- Node.js 18+（附 npm）
- Xcode 命令行工具和 Tauri 依赖（<https://tauri.app/start/prerequisites/>）

### 开发模式

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### 生产构建

```bash
cd cardinal
npm run tauri build
```
