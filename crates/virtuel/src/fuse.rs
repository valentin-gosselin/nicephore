//! Linux: the virtual folder as a read-only FUSE file system. Reads that
//! need a download are answered from a task, so the file system thread
//! never waits on the network.

use crate::arbre::{Arbre, Genre, Inodes, Noeud, Rangement};
use crate::cache::CacheOriginaux;
use api_immich::{AssetDistant, ClientImmich};
use fuser::{
    Config, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo, LockOwner, MountOption,
    OpenAccMode, OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request,
};
use std::ffi::OsStr;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1);

struct SystemeVirtuel {
    arbre: Arc<RwLock<Arbre>>,
    cache: Arc<CacheOriginaux>,
    client: Arc<ClientImmich>,
    runtime: tokio::runtime::Handle,
    uid: u32,
    gid: u32,
    monte_le: SystemTime,
}

fn date(rfc3339: &str) -> SystemTime {
    humantime::parse_rfc3339_weak(rfc3339).unwrap_or(UNIX_EPOCH)
}

impl SystemeVirtuel {
    fn attributs(&self, noeud: &Noeud) -> FileAttr {
        let (kind, perm, taille, mtime, nlink) = match &noeud.genre {
            Genre::Dossier { .. } => (FileType::Directory, 0o555, 0, self.monte_le, 2),
            Genre::Fichier { asset } => {
                // Once downloaded, the real size wins (the server may not know it).
                let reelle = std::fs::metadata(self.cache.chemin(&asset.id)).ok().map(|m| m.len());
                (FileType::RegularFile, 0o444, reelle.or(asset.taille).unwrap_or(0), date(&asset.modifie_le), 1)
            }
        };
        FileAttr {
            ino: INodeNo(noeud.ino),
            size: taille,
            blocks: taille.div_ceil(512),
            atime: mtime,
            mtime,
            ctime: mtime,
            crtime: mtime,
            kind,
            perm,
            nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }
}

impl Filesystem for SystemeVirtuel {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let arbre = self.arbre.read().expect("verrou de l'arbre");
        match name.to_str().and_then(|n| arbre.enfant(parent.0, n)) {
            Some(noeud) => reply.entry(&TTL, &self.attributs(noeud), Generation(0)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let arbre = self.arbre.read().expect("verrou de l'arbre");
        match arbre.noeud(ino.0) {
            Some(noeud) => reply.attr(&TTL, &self.attributs(noeud)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn readdir(&self, _req: &Request, ino: INodeNo, _fh: FileHandle, offset: u64, mut reply: ReplyDirectory) {
        let arbre = self.arbre.read().expect("verrou de l'arbre");
        let Some(noeud) = arbre.noeud(ino.0) else { return reply.error(Errno::ENOENT) };
        if !matches!(noeud.genre, Genre::Dossier { .. }) {
            return reply.error(Errno::ENOTDIR);
        }
        let mut entrees = vec![(ino.0, FileType::Directory, ".".to_string()), (noeud.parent, FileType::Directory, "..".to_string())];
        for enfant in arbre.enfants(ino.0) {
            let genre = if matches!(enfant.genre, Genre::Dossier { .. }) { FileType::Directory } else { FileType::RegularFile };
            entrees.push((enfant.ino, genre, enfant.nom.clone()));
        }
        for (i, (ino, genre, nom)) in entrees.into_iter().enumerate().skip(offset as usize) {
            if reply.add(INodeNo(ino), (i + 1) as u64, genre, nom) {
                break;
            }
        }
        reply.ok();
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        if flags.acc_mode() != OpenAccMode::O_RDONLY {
            return reply.error(Errno::EROFS);
        }
        match self.arbre.read().expect("verrou de l'arbre").noeud(ino.0).map(|n| &n.genre) {
            Some(Genre::Fichier { .. }) => reply.opened(FileHandle(0), FopenFlags::FOPEN_KEEP_CACHE),
            Some(Genre::Dossier { .. }) => reply.error(Errno::EISDIR),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let asset: AssetDistant = match self.arbre.read().expect("verrou de l'arbre").noeud(ino.0).map(|n| &n.genre) {
            Some(Genre::Fichier { asset }) => asset.clone(),
            _ => return reply.error(Errno::ENOENT),
        };
        let (cache, client) = (Arc::clone(&self.cache), Arc::clone(&self.client));
        self.runtime.spawn(async move {
            let chemin = match cache.obtenir(&client, &asset).await {
                Ok(c) => c,
                Err(_) => return reply.error(Errno::EIO),
            };
            let lu = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<u8>> {
                let fichier = std::fs::File::open(chemin)?;
                let mut tampon = vec![0u8; size as usize];
                let mut total = 0;
                while total < tampon.len() {
                    let n = fichier.read_at(&mut tampon[total..], offset + total as u64)?;
                    if n == 0 {
                        break;
                    }
                    total += n;
                }
                tampon.truncate(total);
                Ok(tampon)
            })
            .await;
            match lu {
                Ok(Ok(octets)) => reply.data(&octets),
                _ => reply.error(Errno::EIO),
            }
        });
    }
}

/// A mounted virtual folder; unmounted when dropped.
pub struct Montage {
    session: Mutex<Option<fuser::BackgroundSession>>,
    arbre: Arc<RwLock<Arbre>>,
    inodes: Mutex<Inodes>,
    rangement: Rangement,
    point: PathBuf,
}

impl Montage {
    pub fn point(&self) -> &Path {
        &self.point
    }

    pub fn fichiers(&self) -> usize {
        self.arbre.read().expect("verrou de l'arbre").fichiers()
    }

    /// New content of the selection (periodic refresh).
    pub fn actualiser(&self, assets: &[AssetDistant]) {
        let arbre = Arbre::construire(assets, self.rangement, &mut self.inodes.lock().expect("verrou des inodes"));
        *self.arbre.write().expect("verrou de l'arbre") = arbre;
    }

    pub fn demonter(&self) {
        if let Some(session) = self.session.lock().expect("verrou de la session").take()
            && let Err(e) = session.umount_and_join()
        {
            tracing::warn!("démontage de {} : {e}", self.point.display());
        }
    }
}

impl Drop for Montage {
    fn drop(&mut self) {
        self.demonter();
    }
}

/// Mounts `assets` on `point`, which must be an empty folder (never hides
/// files). A dead mount left by a crash is cleared first.
pub fn monter(
    point: &Path,
    assets: &[AssetDistant],
    rangement: Rangement,
    cache: Arc<CacheOriginaux>,
    client: Arc<ClientImmich>,
    runtime: tokio::runtime::Handle,
) -> Result<Montage, String> {
    let lecture = std::fs::read_dir(point);
    if let Err(e) = &lecture
        && e.raw_os_error() == Some(libc::ENOTCONN)
    {
        // "Transport endpoint is not connected": our own mount, orphaned.
        let _ = std::process::Command::new("fusermount3").arg("-uz").arg(point).status();
    }
    let mut contenu = std::fs::read_dir(point).map_err(|e| format!("dossier {} illisible : {e}", point.display()))?;
    if contenu.next().is_some() {
        return Err("le dossier doit être vide : ses fichiers seraient masqués".into());
    }
    let mut inodes = Inodes::default();
    let arbre = Arc::new(RwLock::new(Arbre::construire(assets, rangement, &mut inodes)));
    let systeme = SystemeVirtuel {
        arbre: Arc::clone(&arbre),
        cache,
        client,
        runtime,
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
        monte_le: SystemTime::now(),
    };
    let mut config = Config::default();
    config.mount_options = vec![
        MountOption::RO,
        MountOption::FSName("nicephore".into()),
        MountOption::Subtype("immich".into()),
        MountOption::DefaultPermissions,
        MountOption::NoAtime,
    ];
    let session = fuser::spawn_mount(systeme, point, &config).map_err(|e| format!("montage impossible : {e}"))?;
    Ok(Montage { session: Mutex::new(Some(session)), arbre, inodes: Mutex::new(inodes), rangement, point: point.to_path_buf() })
}
