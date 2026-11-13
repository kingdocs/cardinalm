> [!NOTE]
> **Searchflux 是 [cardisoft/cardinal](https://github.com/cardisoft/cardinal) 的個人衍生版（基於上游 v0.1.23 二次開發）。**
> 以下為上游原文。本衍生版在上游 master 基礎上增加了：
>
> - **依名稱忽略規則**——「偏好設定」中新增「忽略規則」多行輸入框（位於「忽略路徑」下方），每行一條 .gitignore 風格規則：`*.log`、`.DS_Store`、`~$*` 用於檔案；規則以 `/` 結尾表示只匹配目錄，例如 `.*/` 剪掉所有點開頭的目錄，`node_modules/` 剪掉任意層級的依賴目錄。命中的檔案或目錄子樹不會進入「檔名索引」與「內容索引」，而不只是從搜尋結果中隱藏。檔案規則絕不匹配目錄，目錄規則也絕不匹配檔案；匹配不分大小寫（ASCII）。
> - **關窗即結束**——關閉主視窗時結束應用程式，而不是像上游那樣隱藏到背景。
> - **搜尋引擎 bug 修復**（fork.2）——修了上游 `search-cache` 的 5 個缺陷：一批 FS 事件不再因單一重掃請求被整批丟棄（原先已刪除的檔案會一直留在索引裡）；內容搜尋跳過 iCloud 未下載的佔位檔案（原先會悄悄下載整個雲端硬碟），也跳過 `/dev` 和已非正規檔案的條目（原先可能讓之後所有搜尋卡死）；`content:` 永遠最後執行（原先詞序不同會慢 60 倍）；取消搜尋立即停止派發候選節點。這些修復從 [franciscoxc/cardinal](https://github.com/franciscoxc/cardinal) 手工移植、按本衍生版 v0.1.23 時期的結構適配，**未改介面、未引入更新器、未新增依賴**；每段程式碼都帶 `[port:franciscoxc/<sha>]` 標記，策略見 [`doc/THIRD-PARTY-FIXES.md`](../../doc/THIRD-PARTY-FIXES.md)。
>
> 實作說明：匹配器是獨立、零依賴的 Rust 模組（`fswalk/src/ignore_file.rs`）；規則以指紋形式寫入快取，因此只有規則真正變化才會觸發重建；快取格式版本號與上游錯開以免衝突。本衍生版長期跟隨上游 rebase，改動盡量小而集中。
>
> 發佈版使用本機自簽程式碼簽署憑證簽署（見 `scripts/sign-self-signed.sh`），重編譯不會再讓「完全磁碟存取」授權失效；但不是 Apple Developer ID，首次啟動仍需右鍵「開啟」。
>
> **下載編譯好的版本（Apple Silicon）**：[Releases 頁面](https://github.com/kingdocs/searchflux/releases)

<div align="center">
  <img src="../../cardinal/mac-icon_1024x1024.png" alt="Cardinal icon" width="120" height="120">
  <h1>Cardinal</h1>
  <p>最快又最準的 macOS 檔案搜尋應用程式</p>
  <p>
    <a href="#使用-cardinal">使用 Cardinal</a> ·
    <a href="#建置-cardinal">建置 Cardinal</a>
  </p>
  <img src="UI.gif" alt="Cardinal UI preview" width="720">
</div>

---

[English](../../README.md) · [Español](README.es-ES.md) · [한국어](README.ko-KR.md) · [Русский](README.ru-RU.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [Português](README.pt-BR.md) · [Italiano](README.it-IT.md) · [日本語](README.ja-JP.md) · [Français](README.fr-FR.md) · [Deutsch](README.de-DE.md) · [Українська](README.uk-UA.md) · [العربية](README.ar-SA.md) · [हिन्दी](README.hi-IN.md) · [Türkçe](README.tr-TR.md)

## 使用 Cardinal

### 下載

透過 Homebrew 安裝：

```bash
brew install --cask cardinal-search
```

你也可以從 [GitHub Releases](https://github.com/cardisoft/cardinal/releases/) 取得最新的安裝包。

### 國際化支援

想切換其他語言？點擊狀態列裡的 ⚙️ 按鈕即可即時切換。

### 基礎搜尋語法

Cardinal 現在在經典的子字串/前綴匹配基礎上疊加了 Everything 相容語法：

- `report draft` – 空白代表 `AND`，只會得到同時包含兩個詞的檔案。
- `*.pdf briefing` – 只顯示檔名包含「briefing」的 PDF 結果。
- `*.zip size:>100MB` – 查找大於 100MB 的 ZIP 檔案。
- `in:/Users demo !.psd` – 把搜尋範圍限制在 `/Users`，然後匹配包含 `demo` 但排除 `.psd` 的檔案。
- `tag:ProjectA;ProjectB` – 按 Finder 標籤（macOS）過濾；`;` 表示 `OR`（滿足任一標籤即可）。
- `*.md content:"Bearer "` – 僅篩選包含字串 `Bearer ` 的 Markdown 檔案。
- `"Application Support"` – 使用引號匹配完整片語。
- `brary/Applicat` – 使用 `/` 作為路徑分隔符向下匹配子路徑，例如 `Library/Application Support`。
- `/report` · `draft/` · `/report/` – 在詞首/詞尾添加 `/`，分別強制匹配前綴、後綴或精確檔名，補足 Everything 語法之外的整詞控制。
- `~/**/.DS_Store` – `**` 會深入所有子目錄，在整個家目錄中查找散落的 `.DS_Store` 檔案。

更多支援的運算子（布林組合、資料夾限定、擴充名過濾、正則示例等）請參見 [`search-syntax.zh-TW.md`](search-syntax.zh-TW.md)。

### 鍵盤快捷鍵與預覽

- `Cmd+Shift+Space` – 透過全域快捷鍵開/關 Cardinal 視窗。
- `Cmd+,` – 打開偏好設定。
- `Esc` – 隱藏 Cardinal 視窗。
- `ArrowUp`/`ArrowDown` – 上下移動選取項目。
- `Shift+ArrowUp`/`Shift+ArrowDown` – 擴展選取範圍。
- `Space` – 不離開 Cardinal 即可對目前行執行 Quick Look。
- `Cmd+O` – 打開選中的結果。
- `Cmd+R` – 在 Finder 中定位選中的結果。
- `Cmd+C` – 複製所選檔案到剪貼簿。
- `Cmd+Shift+C` – 複製所選路徑到剪貼簿。
- `Cmd+F` – 焦點回到搜尋框。
- `ArrowUp`/`ArrowDown`（在搜尋框內）– 瀏覽搜尋歷史。

祝你搜尋愉快！

---

## 建置 Cardinal

### 環境需求

- macOS 12+
- Rust 工具鏈
- Node.js 18+（附 npm）
- Xcode 命令列工具和 Tauri 依賴（<https://tauri.app/start/prerequisites/>）

### 開發模式

```bash
cd cardinal
npm run tauri dev -- --release --features dev
```

### 生產建置

```bash
cd cardinal
npm run tauri build
```
