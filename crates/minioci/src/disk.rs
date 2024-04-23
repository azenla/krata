use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use anyhow::{anyhow, Result};
use tokio::{fs, sync::RwLock};

use crate::{name::ImageName, validate::check_valid_digest};

type RepositoryLookupTable = HashMap<String, HashMap<String, String>>;

#[derive(Clone)]
pub struct OciDiskStorage {
    repositories_lock: Arc<RwLock<PathBuf>>,
    blobs_lock: Arc<RwLock<PathBuf>>,
}

impl OciDiskStorage {
    pub async fn new(path: &Path) -> Result<OciDiskStorage> {
        let path = path.to_path_buf();
        fs::create_dir_all(&path).await?;

        let mut repositories_path = path.clone();
        repositories_path.push("repositories.json");

        let mut blobs_path = path.clone();
        blobs_path.push("blobs");

        fs::create_dir_all(&blobs_path).await?;

        Ok(OciDiskStorage {
            repositories_lock: Arc::new(RwLock::new(repositories_path)),
            blobs_lock: Arc::new(RwLock::new(blobs_path)),
        })
    }

    pub async fn repositories_lookup(&self, name: &ImageName) -> Result<Option<String>> {
        let guard = self.repositories_lock.read().await;
        let Some(content) = fs::read_to_string(&*guard).await.ok() else {
            return Ok(None);
        };
        let Some(table) = serde_json::from_str::<RepositoryLookupTable>(&content).ok() else {
            return Ok(None);
        };
        let base = name.base();
        if let Some(items) = table.get(&base) {
            let key = name.normalize().to_string();
            if let Some(digest) = items.get(&key) {
                if check_valid_digest(digest).is_ok() {
                    Ok(Some(digest.clone()))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn repositories_store(&self, name: &ImageName, digest: &str) -> Result<()> {
        check_valid_digest(digest)?;
        let guard = self.repositories_lock.write().await;
        let content = fs::read_to_string(&*guard).await.unwrap_or_else(|_| String::from("{}"));
        let mut table = serde_json::from_str::<RepositoryLookupTable>(&content).unwrap_or_default();
        let base = name.base();
        let entries = table.entry(base).or_default();
        let key: String = name.normalize().to_string();
        entries.insert(key, digest.to_string());
        let mut encoded = serde_json::to_string_pretty(&table)?;
        encoded.push('\n');
        fs::write(&*guard, encoded).await?;
        Ok(())
    }

    pub async fn blob_recall(&self, digest: &str) -> Result<Option<PathBuf>> {
        check_valid_digest(digest)?;
        let guard = self.blobs_lock.read().await;
        let layer_path = digest_storage_path(&*guard, &digest, false).await?;
        if fs::try_exists(&layer_path).await? {
            Ok(Some(layer_path))
        } else {
            Ok(None)
        }
    }

    pub async fn blob_store_file(&self, digest: &str, source: &Path) -> Result<()> {
        check_valid_digest(digest)?;
        let guard = self.blobs_lock.write().await;
        let layer_path = digest_storage_path(&*guard, &digest, true).await?;

        if fs::rename(source, &layer_path).await.is_err() {
            let tmp = append_extension(&layer_path, ".tmp")?;
            fs::copy(source, &tmp).await?;
            fs::rename(tmp, layer_path).await?;
        }
        Ok(())
    }

    pub async fn blob_store_bytes(&self, digest: &str, bytes: &[u8]) -> Result<()> {
        check_valid_digest(digest)?;
        let guard = self.blobs_lock.write().await;
        let blob_path = digest_storage_path(&*guard, &digest, true).await?;
        let tmp = append_extension(&blob_path, ".tmp")?;
        fs::write(&tmp, bytes).await?;
        fs::rename(&tmp, &blob_path).await?;
        Ok(())
    }
}

async fn digest_storage_path(path: &Path, digest: &str, create: bool) -> Result<PathBuf> {
    let mut full = path.to_path_buf();
    let (algorithm, content) = digest.split_once(':').ok_or_else(|| anyhow!("invalid digest"))?;
    full.push(algorithm);
    if create {
        fs::create_dir_all(&full).await?;
    }
    full.push(content);
    Ok(full)
}

fn append_extension(path: &Path, ext: &str) -> Result<PathBuf> {
    let mut path = path.to_path_buf();
    let Some(file_name) = path.file_name() else {
        return Err(anyhow!("file name isn't available"));
    };
    let mut file_name = file_name.to_string_lossy().to_string();
    file_name.push_str(ext);
    path.set_file_name(file_name);
    Ok(path)
}
