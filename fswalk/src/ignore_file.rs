//! 全局忽略规则：按**文件名**匹配，不涉及路径。
//!
//! 两套规则桶，互不干扰（尾斜杠语法与 `.gitignore` 一致）：
//! - 不带 `xxx/` 尾斜杠 → **只匹配文件**。这条是关键：`*.log` 永远不会剪掉一个
//!   名叫 `debug.log` 的文件夹，符合「排除文件类型不影响文件夹索引」。
//! - 带尾斜杠 → **只匹配目录**。`.*/` 排除所有点开头的目录（`.git`、`.venv`、
//!   `.idea`），`node_modules/` 一次搞定所有层的依赖目录，不必逐个写绝对路径。
//!
//! 其他约定：
//! 2. 匹配一律 ASCII 大小写不敏感，`.LOG` 与 `.log` 等价。
//! 3. 含 `/` 的行视为路径规则，本模块跳过——路径排除仍由 `ignore_directories` 负责。
//!
//! 支持的语法（每行一条，`#` 开头为注释，空行忽略）：
//! - `*.log` → 后缀匹配
//! - `~$*` → 前缀匹配
//! - `.DS_Store` → 精确匹配
//! - `报表?.xlsx` → `?` 匹配任意单个字符
//! - `.*/` → 目录规则：点开头的所有目录
//!
//! 实现刻意不引第三方依赖：规则在三类常用形态（精确、后缀、前缀）上走零分配的
//! 切片比较，只有真正含通配符的规则才走回溯匹配。扫描热路径上每 entry 只多两次
//! 字节比较，相对 `readdir` + `stat` 的开销可忽略。

/// 通配符 token。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Token {
    Literal(u8),
    /// `?`：任意单个字节
    One,
    /// `*`：任意长度（含空）字节序列
    Star,
}

/// 单套规则桶（文件或目录其中之一）。
#[derive(Debug, Default, Clone)]
struct RuleSet {
    exact: Vec<Vec<u8>>,
    suffix: Vec<Vec<u8>>,
    prefix: Vec<Vec<u8>>,
    globs: Vec<Vec<Token>>,
}

impl RuleSet {
    fn is_empty(&self) -> bool {
        self.exact.is_empty()
            && self.suffix.is_empty()
            && self.prefix.is_empty()
            && self.globs.is_empty()
    }

    fn len(&self) -> usize {
        self.exact.len() + self.suffix.len() + self.prefix.len() + self.globs.len()
    }

    /// 规则集是集合：排序去重不改变语义，却让 `digest()` 与书写顺序无关。
    /// 否则用户把两行调换位置就会触发一次整盘重扫。
    fn sort_dedup(&mut self) {
        self.exact.sort_unstable();
        self.exact.dedup();
        self.suffix.sort_unstable();
        self.suffix.dedup();
        self.prefix.sort_unstable();
        self.prefix.dedup();
        self.globs.sort_unstable();
        self.globs.dedup();
    }

    fn digest_into(&self, hash: &mut u64) {
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        fn feed(hash: &mut u64, bytes: &[u8]) {
            for byte in bytes {
                *hash ^= u64::from(*byte);
                *hash = hash.wrapping_mul(PRIME);
            }
        }

        // 分隔符确保 ["a","b"] 与 ["ab"] 不会撞。
        // 长度用 64 位小端：规则数超过 255 条时不会因 u8 截断而撞指纹。
        for bucket in [&self.exact, &self.suffix, &self.prefix] {
            feed(hash, &(bucket.len() as u64).to_le_bytes());
            for pattern in bucket.iter() {
                feed(hash, pattern);
                feed(hash, &[0]);
            }
        }
        feed(hash, &(self.globs.len() as u64).to_le_bytes());
        for glob in self.globs.iter() {
            for token in glob.iter() {
                let byte = match token {
                    Token::Literal(literal) => *literal,
                    Token::One => 0x3f,
                    Token::Star => 0x2a,
                };
                feed(hash, &[byte]);
            }
            feed(hash, &[0]);
        }
    }

    fn matches(&self, name: &[u8]) -> bool {
        if self
            .exact
            .iter()
            .any(|pattern| name.eq_ignore_ascii_case(pattern))
        {
            return true;
        }
        if self.suffix.iter().any(|suffix| {
            name.len() >= suffix.len() && name[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
        }) {
            return true;
        }
        if self.prefix.iter().any(|prefix| {
            name.len() >= prefix.len() && name[..prefix.len()].eq_ignore_ascii_case(prefix)
        }) {
            return true;
        }
        self.globs.iter().any(|glob| glob_match(glob, name))
    }
}

/// 编译后的规则集合。构建一次，扫描期间只读。
#[derive(Debug, Default, Clone)]
pub struct FileNameRules {
    /// 不带尾斜杠的规则：只对文件生效。
    files: RuleSet,
    /// 带尾斜杠的规则：只对目录生效。
    dirs: RuleSet,
}

impl FileNameRules {
    /// 从设置界面的多行文本构建规则集。
    pub fn from_lines<I, S>(lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut rules = Self::default();
        for line in lines {
            if let Some(rule) = Rule::parse(line.as_ref()) {
                rule.push_into(&mut rules);
            }
        }
        rules.files.sort_dedup();
        rules.dirs.sort_dedup();
        rules
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.dirs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.files.len() + self.dirs.len()
    }

    /// 规则集指纹，用于持久化缓存的一致性校验。
    ///
    /// 覆盖「语义」（编译后的规则，含目录规则），不覆盖书写顺序、重复行与注释：
    /// `from_lines` 已对每个桶排序去重，因此调换两行位置不会导致整盘重扫，
    /// 但改动任何一条规则（包括把 `.*` 改成 `.*/`）都会触发重建。
    pub fn digest(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        let mut hash = OFFSET;
        self.files.digest_into(&mut hash);
        // 两个桶之间加标记，避免「文件规则 A + 空目录规则」与「空文件规则 + 目录规则 A」撞指纹。
        hash ^= 0x9e37_79b9_7f4a_7c15;
        self.dirs.digest_into(&mut hash);
        hash
    }

    /// 判断文件名是否命中**文件**规则。
    ///
    /// 保留此名作为 `is_ignored_file` 的别名，历史上只有这一个入口。
    pub fn is_ignored(&self, name: &std::ffi::OsStr) -> bool {
        self.is_ignored_file(name)
    }

    /// 判断文件名是否命中**文件**规则。调用方须保证传入的是文件的名字。
    pub fn is_ignored_file(&self, name: &std::ffi::OsStr) -> bool {
        if self.files.is_empty() {
            return false;
        }
        self.files
            .matches(std::os::unix::ffi::OsStrExt::as_bytes(name))
    }

    /// 判断目录名是否命中**目录**规则（带尾斜杠的那些）。调用方须保证传入的是目录的名字。
    pub fn is_ignored_dir(&self, name: &std::ffi::OsStr) -> bool {
        if self.dirs.is_empty() {
            return false;
        }
        self.dirs
            .matches(std::os::unix::ffi::OsStrExt::as_bytes(name))
    }
}

/// 匹配目标：同一条写法只作用于一类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    File,
    Dir,
}

enum Matcher {
    Exact(Vec<u8>),
    Suffix(Vec<u8>),
    Prefix(Vec<u8>),
    Glob(Vec<Token>),
}

struct Rule {
    target: Target,
    matcher: Matcher,
}

impl Rule {
    fn parse(raw: &str) -> Option<Self> {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        // 尾斜杠 = 只匹配目录，与 .gitignore 一致。
        let (pattern_str, target) = match line.strip_suffix('/') {
            Some(rest) => (rest.trim_end(), Target::Dir),
            None => (line, Target::File),
        };
        // 单写一个 "/" 没有含义。
        if pattern_str.is_empty() {
            return None;
        }
        // 路径规则不属于文件名排除，跳过，避免误伤目录。
        if pattern_str.contains('/') {
            return None;
        }
        let matcher = Self::compile(pattern_str.as_bytes())?;
        Some(Self { target, matcher })
    }

    fn compile(pattern: &[u8]) -> Option<Matcher> {
        let wildcards = pattern
            .iter()
            .filter(|byte| matches!(byte, b'*' | b'?'))
            .count();
        if wildcards == 0 {
            return Some(Matcher::Exact(pattern.to_vec()));
        }
        // 长度必须大于 1：单独的 `*` 若归入 Suffix 会得到空后缀，
        // 虽然结果同样是"匹配所有"，但让空模式参与字节比较没有意义。
        if wildcards == 1
            && pattern.len() > 1
            && pattern.first() == Some(&b'*')
            && !pattern[1..].contains(&b'?')
        {
            return Some(Matcher::Suffix(pattern[1..].to_vec()));
        }
        if wildcards == 1
            && pattern.len() > 1
            && pattern.last() == Some(&b'*')
            && !pattern[..pattern.len() - 1].contains(&b'?')
        {
            return Some(Matcher::Prefix(pattern[..pattern.len() - 1].to_vec()));
        }
        Some(Matcher::Glob(
            pattern
                .iter()
                .map(|byte| match byte {
                    b'*' => Token::Star,
                    b'?' => Token::One,
                    literal => Token::Literal(*literal),
                })
                .collect(),
        ))
    }

    fn push_into(self, rules: &mut FileNameRules) {
        let bucket = match self.target {
            Target::File => &mut rules.files,
            Target::Dir => &mut rules.dirs,
        };
        match self.matcher {
            Matcher::Exact(pattern) => bucket.exact.push(pattern),
            Matcher::Suffix(pattern) => bucket.suffix.push(pattern),
            Matcher::Prefix(pattern) => bucket.prefix.push(pattern),
            Matcher::Glob(tokens) => bucket.globs.push(tokens),
        }
    }
}

/// 线性回溯的通配符匹配：`*` / `?` / 字面量，ASCII 大小写不敏感。
fn glob_match(pattern: &[Token], name: &[u8]) -> bool {
    let mut p = 0usize;
    let mut s = 0usize;
    let mut star: Option<usize> = None;
    let mut mark = 0usize;

    while s < name.len() {
        match pattern.get(p) {
            Some(Token::Star) => {
                star = Some(p);
                mark = s;
                p += 1;
            }
            Some(Token::One) => {
                p += 1;
                s += 1;
            }
            Some(Token::Literal(literal)) if literal.eq_ignore_ascii_case(&name[s]) => {
                p += 1;
                s += 1;
            }
            _ => match star {
                Some(star_pos) => {
                    p = star_pos + 1;
                    mark += 1;
                    s = mark;
                }
                None => return false,
            },
        }
    }

    while matches!(pattern.get(p), Some(Token::Star)) {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn rules(lines: &[&str]) -> FileNameRules {
        FileNameRules::from_lines(lines.iter().copied())
    }

    #[test]
    fn suffix_matches_case_insensitively() {
        let rules = rules(&["*.log"]);
        assert!(rules.is_ignored_file(OsStr::new("debug.log")));
        assert!(rules.is_ignored_file(OsStr::new("DEBUG.LOG")));
        assert!(rules.is_ignored_file(OsStr::new("a.b.Log")));
        assert!(!rules.is_ignored_file(OsStr::new("debug.log.txt")));
        assert!(!rules.is_ignored_file(OsStr::new("log")));
    }

    #[test]
    fn exact_matches_only_same_name() {
        let rules = rules(&[".DS_Store", "Thumbs.db"]);
        assert!(rules.is_ignored(OsStr::new(".DS_Store")));
        assert!(rules.is_ignored(OsStr::new("thumbs.DB")));
        assert!(!rules.is_ignored(OsStr::new("my.DS_Store")));
    }

    #[test]
    fn prefix_matches_office_lock_files() {
        let rules = rules(&["~$*"]);
        assert!(rules.is_ignored(OsStr::new("~$合同.docx")));
        assert!(!rules.is_ignored(OsStr::new("合同.docx")));
    }

    #[test]
    fn question_mark_matches_single_char() {
        let rules = rules(&["报表?.xlsx"]);
        assert!(rules.is_ignored(OsStr::new("报表1.xlsx")));
        assert!(!rules.is_ignored(OsStr::new("报表10.xlsx")));
    }

    #[test]
    fn mid_pattern_star_backtracks() {
        let rules = rules(&["a*b*c.log"]);
        assert!(rules.is_ignored(OsStr::new("aXXbYYc.log")));
        assert!(rules.is_ignored(OsStr::new("abc.LOG")));
        assert!(!rules.is_ignored(OsStr::new("aXXbYY.log")));
    }

    #[test]
    fn skips_comments_blank_and_path_rules() {
        let rules = rules(&[
            "",
            "   ",
            "# 这是注释",
            "/Volumes/",
            "~/Library/Caches/",
            "a/b",
            "*.tmp",
            "node_modules/",
        ]);
        // 只有 `*.tmp` 与 `node_modules/` 是合法规则，其余被跳过。
        assert_eq!(rules.len(), 2);
        assert!(rules.is_ignored_file(OsStr::new("x.tmp")));
        assert!(rules.is_ignored_dir(OsStr::new("node_modules")));
    }

    #[test]
    fn empty_ruleset_never_matches() {
        let rules = rules(&[]);
        assert!(rules.is_empty());
        assert!(!rules.is_ignored_file(OsStr::new("debug.log")));
        assert!(!rules.is_ignored_dir(OsStr::new(".git")));
    }

    #[test]
    fn star_matches_whole_name() {
        let rules = rules(&["*"]);
        assert!(rules.is_ignored_file(OsStr::new("anything")));
        assert!(rules.is_ignored_file(OsStr::new(".hidden")));
    }

    #[test]
    fn digest_is_order_insensitive_and_dedups() {
        let a = rules(&["*.log", "*.tmp", ".DS_Store", "# 注释"]);
        let b = rules(&[".DS_Store", "*.tmp", "*.log", "*.log"]);
        // 调换顺序、重复一行、带注释，都不应触发整盘重扫。
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a.len(), 3);
        // 语义不同的规则集必须指纹不同。
        assert_ne!(a.digest(), rules(&["*.log"]).digest());
        // ["a","b"] 与 ["ab"] 不能撞。
        assert_ne!(rules(&["a", "b"]).digest(), rules(&["ab"]).digest());
    }

    #[test]
    fn dir_rule_matches_only_directories() {
        let rules = rules(&[".*/"]);
        assert!(rules.is_ignored_dir(OsStr::new(".git")));
        assert!(rules.is_ignored_dir(OsStr::new(".DS_Store"))); // 文件夹也叫这名字
        assert!(rules.is_ignored_dir(OsStr::new(".config")));
        // 文件方向不应命中：目录规则不参与文件判断。
        assert!(!rules.is_ignored_file(OsStr::new(".env")));
        assert!(!rules.is_ignored_file(OsStr::new(".git")));
    }

    /// 这条是「忽略文件类型不影响文件夹索引」的保护锁：
    /// 老用户只写了 `*.log`，绝不能因为新语法让 `debug.log` 文件夹消失。
    #[test]
    fn file_rule_never_matches_directory() {
        let rules = rules(&["*.log", ".DS_Store"]);
        assert!(rules.is_ignored_file(OsStr::new("debug.log")));
        assert!(!rules.is_ignored_dir(OsStr::new("debug.log")));
        assert!(!rules.is_ignored_dir(OsStr::new(".DS_Store")));
    }

    #[test]
    fn exact_dir_rule_matches_named_folder() {
        let rules = rules(&["node_modules/", ".git/"]);
        assert!(rules.is_ignored_dir(OsStr::new("node_modules")));
        assert!(rules.is_ignored_dir(OsStr::new("NODE_MODULES")));
        assert!(rules.is_ignored_dir(OsStr::new(".git")));
        assert!(!rules.is_ignored_dir(OsStr::new("node_modules_backup")));
        // 同名文件不受目录规则影响。
        assert!(!rules.is_ignored_file(OsStr::new("node_modules")));
    }

    #[test]
    fn same_pattern_can_target_both_kinds() {
        let rules = rules(&[".*", ".*/"]);
        assert!(rules.is_ignored_file(OsStr::new(".DS_Store")));
        assert!(rules.is_ignored_dir(OsStr::new(".git")));
        assert_eq!(rules.len(), 2);
    }
}
