//! Virtual files: a selection of the server (album, person, period...)
//! shown as an ordinary read-only folder, each original downloaded only
//! when a program reads it, then kept in a capped cache.
//!
//! Linux: a FUSE file system. Windows: the Cloud Files API (placeholders),
//! see `windows.rs`.

pub mod arbre;
pub mod cache;
#[cfg(target_os = "linux")]
mod fuse;
#[cfg(windows)]
pub mod windows;

pub use arbre::{Arbre, Genre, Inodes, Noeud, Rangement};
pub use cache::CacheOriginaux;
#[cfg(target_os = "linux")]
pub use fuse::{Montage, monter};

/// Windows: originals come from the cache, downloaded on the async runtime.
#[cfg(windows)]
pub struct SourceCache {
    pub cache: std::sync::Arc<CacheOriginaux>,
    pub client: std::sync::Arc<api_immich::ClientImmich>,
    pub runtime: tokio::runtime::Handle,
}

#[cfg(windows)]
impl windows::Source for SourceCache {
    fn original(&self, asset: &api_immich::AssetDistant) -> Result<std::path::PathBuf, String> {
        self.runtime.block_on(self.cache.obtenir(&self.client, asset))
    }
}
