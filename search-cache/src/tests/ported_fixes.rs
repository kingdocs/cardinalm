//! 移植自 `franciscoxc/cardinal` 的引擎 bug 修复的回归测试。
//!
//! 每个测试带 `[port:franciscoxc/<sha>]` 标记。rebase 到官方后若这些测试挂了，
//! 优先怀疑「官方也修了同一处」—— 见 `UPSTREAM-SYNC.md` 第 10.3 节的判定规则。

use super::prelude::*;
use crate::query::{UnreadReason, content_unread_reason, is_dataless_flags};
use std::{
    os::{fd::AsRawFd, macos::fs::MetadataExt},
    path::Path,
};

/// [port:franciscoxc/8bc9bae5 + 6cf78bdb] 守卫函数本身：/dev 一律不读，
/// 目录 / FIFO 等非常规文件不读，普通本地文件放行。
#[test]
fn content_unread_reason_rejects_dev_and_non_regular_files() {
    assert_eq!(
        content_unread_reason(Path::new("/dev/fd/0")),
        Some(UnreadReason::NotAFile),
        "descriptor mirrors must never be read"
    );
    // Path::starts_with 按组件匹配，不能误伤名字以 dev 开头的目录。
    assert_eq!(
        content_unread_reason(Path::new("/devdocs/plain.txt")),
        None,
        "a sibling of /dev is not inside /dev"
    );

    let tmp = TempDir::new("unread_reason").unwrap();
    let dir = tmp.path();
    let file = dir.join("local.txt");
    fs::write(&file, b"needle").unwrap();
    fs::create_dir_all(dir.join("a-folder")).unwrap();

    assert_eq!(
        content_unread_reason(&file),
        None,
        "an ordinary local file is opened as usual"
    );
    assert_eq!(
        content_unread_reason(&dir.join("a-folder")),
        Some(UnreadReason::NotAFile),
        "a directory is not a regular file"
    );

    let fifo = dir.join("a-fifo");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo runs");
    assert!(made.success());
    assert_eq!(
        content_unread_reason(&fifo),
        Some(UnreadReason::NotAFile),
        "a FIFO would block forever on open"
    );
}

/// [port:franciscoxc/6cf78bdb] 索引时是普通文件、搜索时变成 FIFO，
/// 内容搜索必须跳过它而不是卡死。放线程里跑，超时即判失败（否则整个测试集会挂住）。
#[test]
fn content_search_leaves_alone_what_is_no_longer_a_regular_file() {
    let tmp = TempDir::new("content_not_a_file").unwrap();
    let dir = tmp.path();
    let path = dir.join("was-a-file.txt");
    fs::write(&path, b"needle").unwrap();
    fs::write(dir.join("still-a-file.txt"), b"needle").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    fs::remove_file(&path).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(&path)
        .status()
        .expect("mkfifo runs");
    assert!(made.success());

    let (answer_tx, answer_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = answer_tx.send(cache.search("content:needle").map(|hits| hits.len()));
    });
    let answer = answer_rx.recv_timeout(std::time::Duration::from_secs(10));
    if answer.is_err() {
        // 万一真的阻塞了，给 FIFO 一个写端让读端退出，别把测试进程挂死。
        let _ = fs::OpenOptions::new().write(true).open(&path);
    }

    assert_eq!(
        answer.expect("the search has to come back").unwrap(),
        1,
        "the regular file matches; the pipe is never opened"
    );
}

/// [port:franciscoxc/6cf78bdb] /dev/fd/N 在索引它的进程里像普通文件，
/// 在搜索进程里可能是 socket。守卫必须按路径挡住它，且不影响原路径本身。
#[test]
fn content_is_never_read_through_dev() {
    use std::io::Seek;

    let tmp = TempDir::new("content_through_dev").unwrap();
    let path = tmp.path().join("held-open.txt");
    fs::write(&path, b"a needle in a file this process holds open").unwrap();
    let mut held = fs::File::open(&path).unwrap();
    let mirror = PathBuf::from(format!("/dev/fd/{}", held.as_raw_fd()));
    assert!(
        fs::symlink_metadata(&mirror).unwrap().file_type().is_file(),
        "the mirror looks like a regular file, so its type alone would let it through"
    );

    assert_eq!(
        content_unread_reason(&mirror),
        Some(UnreadReason::NotAFile),
        "a descriptor mirror is off limits"
    );
    assert_eq!(
        content_unread_reason(&path),
        None,
        "by its own path the file is searched as usual"
    );
    assert_eq!(held.stream_position().unwrap(), 0, "no read happened at all");
}

/// [port:franciscoxc/8bc9bae5] SF_DATALESS 标志位的判定必须独立可测：
/// 普通文件在本机造不出真正的云占位文件（chflags 会静默不生效），
/// 端到端测试会跳过，所以这一条负责覆盖标志逻辑本身。
#[test]
fn dataless_flag_is_recognised() {
    const SF_DATALESS: u32 = 0x4000_0000;
    assert!(!is_dataless_flags(0), "a local file is not a placeholder");
    assert!(
        is_dataless_flags(SF_DATALESS),
        "SF_DATALESS alone marks a placeholder"
    );
    assert!(
        is_dataless_flags(SF_DATALESS | 0x0000_0020),
        "other flags may be set at the same time"
    );
    assert!(
        !is_dataless_flags(0x2000_0000),
        "an unrelated high bit must not be mistaken for it"
    );
}

/// [port:franciscoxc/8bc9bae5] iCloud 未下载的占位文件不能被内容搜索打开。
/// 用 `chflags dataless` 造一个；普通文件上该命令未必生效，读回标志确认，
/// 不生效就跳过（否则测试会在不证明任何东西的情况下通过）。
/// 标志位判定本身另由 `dataless_flag_is_recognised` 无条件覆盖。
#[test]
fn content_search_does_not_open_files_that_would_be_downloaded() {
    const SF_DATALESS: u32 = 0x4000_0000;

    let tmp = TempDir::new("dataless_content").unwrap();
    let dir = tmp.path();
    fs::write(dir.join("local.txt"), b"contiene la palabra factura").unwrap();
    fs::write(dir.join("cloud.txt"), b"contiene la palabra factura").unwrap();

    let _ = std::process::Command::new("chflags")
        .arg("dataless")
        .arg(dir.join("cloud.txt"))
        .status();
    let marked = fs::symlink_metadata(dir.join("cloud.txt"))
        .map(|m| m.st_flags() & SF_DATALESS != 0)
        .unwrap_or(false);
    if !marked {
        eprintln!("SKIP: this machine will not let a plain file be marked dataless");
        return;
    }

    assert_eq!(
        content_unread_reason(&dir.join("cloud.txt")),
        Some(UnreadReason::InTheCloud),
        "the placeholder is recognised without being opened"
    );

    let mut cache = SearchCache::walk_fs(dir);
    let hits = cache.search("content:factura").unwrap();
    let names: Vec<String> = hits
        .iter()
        .filter_map(|i| cache.node_path(*i))
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    assert_eq!(
        names,
        vec!["local.txt".to_string()],
        "the local file matches; the one that would have to be downloaded is left alone"
    );
}

/// [port:franciscoxc/fe8e88e2] 被取消的内容搜索要立刻停止派发文件，
/// 而不是把剩下的每个节点都构建一遍路径。派发侧原先完全不检查取消，
/// 628 万节点索引上实测要 10.3 秒才能开始下一次搜索。
#[test]
fn a_cancelled_content_search_stops_handing_out_files() {
    let tmp = TempDir::new("content_candidates_cancelled").unwrap();
    let dir = tmp.path();
    fs::create_dir_all(dir.join("notes")).unwrap();
    fs::write(dir.join("notes/a.txt"), b"x").unwrap();
    fs::write(dir.join("notes/b.txt"), b"x").unwrap();
    fs::write(dir.join("c.txt"), b"x").unwrap();

    let cache = SearchCache::walk_fs(dir);
    let everything = cache
        .search_empty(CancellationToken::noop())
        .expect("noop token should not cancel");

    let live: Vec<_> = cache
        .content_candidates(everything.clone(), CancellationToken::noop())
        .collect();
    assert_eq!(live.len(), 3, "the three files, not the folders: {live:?}");

    // 被更新的搜索取代，就是「取消」的全部含义。
    let token = CancellationToken::new_search();
    let _ = CancellationToken::new_search();
    assert_eq!(
        cache.content_candidates(everything, token).count(),
        0,
        "a search nobody is waiting for must not build another path"
    );
}
