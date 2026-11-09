use crate::{SlabIndex, SlabNode, ThinSlab};
use fswalk::FileNameRules;
use std::{
    ffi::OsStr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug)]
pub struct FileNodes {
    path: PathBuf,
    ignore_paths: Vec<PathBuf>,
    include_paths: Vec<PathBuf>,
    /// 全局忽略文件类型规则（按文件名匹配，只作用于文件）。
    file_name_rules: Arc<FileNameRules>,
    slab: ThinSlab<SlabNode>,
    root: SlabIndex,
}

impl FileNodes {
    pub(crate) fn new(
        path: PathBuf,
        ignore_paths: Vec<PathBuf>,
        include_paths: Vec<PathBuf>,
        file_name_rules: Arc<FileNameRules>,
        slab: ThinSlab<SlabNode>,
        root: SlabIndex,
    ) -> Self {
        Self {
            path,
            ignore_paths,
            include_paths,
            file_name_rules,
            slab,
            root,
        }
    }

    pub(crate) fn root(&self) -> SlabIndex {
        self.root
    }

    pub fn node_path(&self, index: SlabIndex) -> Option<PathBuf> {
        let mut current = index;
        let mut segments = vec![];
        while let Some(parent) = self.slab.get(current)?.parent() {
            segments.push(self.slab.get(current)?.name());
            current = parent;
        }
        Some(
            std::iter::once("/")
                .chain(segments.into_iter().rev())
                .map(OsStr::new)
                .collect(),
        )
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn ignore_paths(&self) -> &Vec<PathBuf> {
        &self.ignore_paths
    }

    pub(crate) fn include_paths(&self) -> &Vec<PathBuf> {
        &self.include_paths
    }

    pub(crate) fn file_name_rules(&self) -> &Arc<FileNameRules> {
        &self.file_name_rules
    }

    pub(crate) fn file_name_rules_digest(&self) -> u64 {
        self.file_name_rules.digest()
    }

    pub(crate) fn take_slab(&mut self) -> ThinSlab<SlabNode> {
        std::mem::take(&mut self.slab)
    }

    pub(crate) fn put_slab(&mut self, slab: ThinSlab<SlabNode>) {
        self.slab = slab;
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        PathBuf,
        Vec<PathBuf>,
        Vec<PathBuf>,
        u64,
        SlabIndex,
        ThinSlab<SlabNode>,
    ) {
        let Self {
            path,
            ignore_paths,
            include_paths,
            file_name_rules,
            slab,
            root,
        } = self;
        let digest = file_name_rules.digest();
        (path, ignore_paths, include_paths, digest, root, slab)
    }
}

impl Deref for FileNodes {
    type Target = ThinSlab<SlabNode>;

    fn deref(&self) -> &Self::Target {
        &self.slab
    }
}

impl DerefMut for FileNodes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slab
    }
}
