//! Test-build-only fixtures. No environment override is compiled into normal builds.
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct QaFixture {
    root: PathBuf,
}
impl QaFixture {
    pub fn open(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        let marker = root.join("TEST-ONLY");
        if std::fs::symlink_metadata(&marker)?.file_type().is_symlink()
            || std::fs::read(&marker)? != b"YoYoVideo updater QA fixture v1\n"
        {
            return Err(io::Error::other("A marked, isolated updater QA root is required"));
        }
        Ok(Self { root })
    }
    pub fn from_env() -> io::Result<Self> {
        let root = std::env::var_os("YOYOVIDEO_UPDATER_QA_ROOT")
            .ok_or_else(|| io::Error::other("QA build requires YOYOVIDEO_UPDATER_QA_ROOT"))?;
        Self::open(Path::new(&root))
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn public_key(&self) -> io::Result<String> {
        let file = self.root.join("public.key");
        let metadata = std::fs::symlink_metadata(&file)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 4096 {
            return Err(io::Error::other("Invalid QA public key file"));
        }
        std::fs::read_to_string(file)
    }
    pub fn source_file(&self, name: &str) -> io::Result<PathBuf> {
        if name.is_empty()
            || name.len() > 240
            || name.starts_with('.')
            || name.contains("..")
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(io::Error::other("Invalid fixture asset name"));
        }
        let source = self.root.join("source").canonicalize()?;
        let file = source.join(name).canonicalize()?;
        if !source.starts_with(&self.root)
            || file.parent() != Some(source.as_path())
            || !file.is_file()
        {
            return Err(io::Error::other("Fixture source escaped its directory"));
        }
        Ok(file)
    }
}
