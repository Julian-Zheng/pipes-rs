use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use memmap2::{Mmap, MmapMut};

use crate::error::{Result, ShmError};

/// Default directory where dora-lite shared memory files are placed.
pub fn default_shm_dir() -> PathBuf {
    std::env::var("DORA_SHM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("dora_shm"))
}

/// Resolves an shm_id into an absolute file path.
pub fn resolve_shm_path(shm_id: &str) -> PathBuf {
    let dir = default_shm_dir();
    dir.join(format!("{}.shm", shm_id))
}

/// A read-write memory mapped shared memory region.
pub struct MmapMutRegion {
    path: PathBuf,
    _file: File,
    mmap: MmapMut,
    size: usize,
    auto_clean: bool,
}

impl MmapMutRegion {
    /// Creates a new shared memory region file and maps it read-write.
    pub fn create(shm_id: &str, size: usize, auto_clean: bool) -> Result<Self> {
        let path = resolve_shm_path(shm_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        file.set_len(size as u64)?;

        // Safety: We created and set the file length immediately above.
        let mmap = unsafe { MmapMut::map_mut(&file)? };

        Ok(Self {
            path,
            _file: file,
            mmap,
            size,
            auto_clean,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[..]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.mmap[..]
    }

    pub fn flush(&self) -> Result<()> {
        self.mmap.flush()?;
        Ok(())
    }
}

impl Drop for MmapMutRegion {
    fn drop(&mut self) {
        if self.auto_clean && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// A read-only memory mapped shared memory region for consumer nodes.
pub struct MmapViewRegion {
    path: PathBuf,
    _file: File,
    mmap: Mmap,
    size: usize,
}

impl MmapViewRegion {
    /// Opens an existing shared memory region file read-only.
    pub fn open(shm_id: &str) -> Result<Self> {
        let path = resolve_shm_path(shm_id);
        if !path.exists() {
            return Err(ShmError::NotFound(path.to_string_lossy().to_string()));
        }

        let file = OpenOptions::new().read(true).open(&path)?;
        let metadata = file.metadata()?;
        let size = metadata.len() as usize;

        // Safety: We mapped an existing file read-only.
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(Self {
            path,
            _file: file,
            mmap,
            size,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[..]
    }

    /// Borrows a subslice zero-copy given offset and length.
    pub fn slice(&self, offset: u64, len: u64) -> Result<&[u8]> {
        let start = offset as usize;
        let end = (offset + len) as usize;

        if end > self.size {
            return Err(ShmError::OutOfBounds {
                offset,
                len,
                total_size: self.size as u64,
            });
        }

        Ok(&self.mmap[start..end])
    }
}
