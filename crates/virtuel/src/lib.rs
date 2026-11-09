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

pub use arbre::{Arbre, Genre, Inodes, Noeud, Rangement};
pub use cache::CacheOriginaux;
#[cfg(target_os = "linux")]
pub use fuse::{Montage, monter};
