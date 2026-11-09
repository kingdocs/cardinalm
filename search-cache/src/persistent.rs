use crate::{SlabIndex, SlabNode, ThinSlab, name_index::SortedSlabIndices};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    thread::available_parallelism,
    time::Instant,
};
use tracing::info;
use typed_num::Num;

/// 个人自用版：上游为 6，本改造在 `PersistentStorage` 里新增了
/// `ignore_file_rules_digest` 字段，格式已不兼容。
///
/// 刻意取 1007 而不是 7：上游若把版本升到 7、8、9，也不会与本地的
/// 「版本号相同但字段不同」撞车（那种情况反序列化会失败在字段层，报错更晦涩）。
/// 版本号不一致时 `read_cache_from_file` 会直接放弃缓存并触发全量重建，属正常降级。
const LSF_VERSION: i64 = 1007;

#[derive(Serialize, Deserialize)]
pub struct PersistentStorage {
    pub version: Num<LSF_VERSION>,
    /// The last event id of the cache.
    pub last_event_id: u64,
    /// Root file path of the cache
    pub path: PathBuf,
    /// Ignore paths
    pub ignore_paths: Vec<PathBuf>,
    /// Paths to include even when they fall under an ignored directory.
    pub include_paths: Vec<PathBuf>,
    /// Fingerprint of the "ignore file types" rules that produced this cache.
    /// 规则变了指纹就变，旧缓存自动失效并触发全量重建。
    pub ignore_file_rules_digest: u64,
    /// Root index of the slab
    pub slab_root: SlabIndex,
    pub slab: ThinSlab<SlabNode>,
    pub name_index: BTreeMap<Box<str>, SortedSlabIndices>,
    /// The number of rescans emitted before this snapshot.
    pub rescan_count: u64,
}

pub fn read_cache_from_file(path: &Path) -> Result<PersistentStorage> {
    let cache_decode_time = Instant::now();
    let mut bytes = vec![0u8; 4 * 1024];
    let input = File::open(path).context("Failed to open cache file")?;
    let input = zstd::Decoder::new(input).context("Failed to create zstd decoder")?;
    let mut input = BufReader::new(input);
    let storage: PersistentStorage = postcard::from_io((&mut input, &mut bytes))
        .context("Failed to decode cache, maybe the cache is corrupted")?
        .0;
    info!("Cache decode time: {:?}", cache_decode_time.elapsed());
    Ok(storage)
}

pub fn write_cache_to_file(path: &Path, storage: &PersistentStorage) -> Result<()> {
    let cache_encode_time = Instant::now();
    let _ = fs::create_dir_all(path.parent().unwrap());
    let tmp_path = &path.with_extension(".sctmp");
    {
        let output = File::create(tmp_path).context("Failed to create cache file")?;
        let mut output = zstd::Encoder::new(output, 6).context("Failed to create zstd encoder")?;
        output
            .multithread(available_parallelism().map(|x| x.get() as u32).unwrap_or(4))
            .context("Failed to create parallel zstd encoder")?;
        let output = output.auto_finish();
        let mut output = BufWriter::new(output);
        postcard::to_io(storage, &mut output).context("Failed to encode cache")?;
    }
    fs::rename(tmp_path, path).context("Failed to rename cache file")?;
    info!("Cache encode time: {:?}", cache_encode_time.elapsed());
    info!(
        "Cache size: {} MB",
        fs::symlink_metadata(path)
            .context("Failed to get cache file metadata")?
            .len() as f32
            / 1024.
            / 1024.
    );
    Ok(())
}
