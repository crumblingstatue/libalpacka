#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![expect(clippy::missing_errors_doc)]

pub mod dep;
mod pkgdesc;
pub use pkgdesc::{Depend, InstallReason, OptDepend, PkgDesc, ReqCmp, Validation};

use {
    flate2::read::GzDecoder,
    std::{io::Read, path::Path},
    tar::Archive,
};

pub struct Pkg {
    pub desc: PkgDesc,
    pub files: Vec<Box<str>>,
}

const SUPPORTED_DB_VERSION: &str = "9";

#[derive(Debug, thiserror::Error)]
pub enum DbReadError {
    #[error("Io error")]
    Io(#[from] std::io::Error),
    #[error("Supported DB version mismatch. Expected: {supported}, got {got}")]
    DbVerMismatch {
        supported: &'static str,
        got: String,
    },
}

pub fn read_local_db() -> Result<Vec<Pkg>, DbReadError> {
    let mut pkgs = vec![];
    let local_db_root = Path::new("/var/lib/pacman/local/");
    let db_ver = std::fs::read_to_string(local_db_root.join("ALPM_DB_VERSION"))?;
    if db_ver.trim() != SUPPORTED_DB_VERSION {
        return Err(DbReadError::DbVerMismatch {
            supported: SUPPORTED_DB_VERSION,
            got: db_ver,
        });
    }
    for entry in std::fs::read_dir(local_db_root)? {
        let entry = entry?;
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let install_script = entry.path().join("install").exists();
        let desc = PkgDesc::parse(
            &std::fs::read_to_string(entry.path().join("desc"))?,
            install_script,
        );
        pkgs.push(Pkg {
            desc,
            files: std::fs::read_to_string(entry.path().join("files"))?
                .lines()
                .skip(1)
                .take_while(|l| !l.is_empty())
                .map(From::from)
                .collect(),
        });
    }
    Ok(pkgs)
}

pub fn read_syncdb(name: &str) -> Result<Vec<Pkg>, DbReadError> {
    let mut pkgs = vec![];
    let dec = GzDecoder::new(std::fs::File::open(format!(
        "/var/lib/pacman/sync/{name}.db"
    ))?);
    let mut ar = Archive::new(dec);

    for en in ar.entries()? {
        let mut en = en?;
        let mut s = String::new();
        if en.path()?.file_name() == Some("desc".as_ref()) {
            en.read_to_string(&mut s)?;
            pkgs.push(Pkg {
                desc: PkgDesc::parse(&s, false),
                files: Vec::new(),
            });
        }
    }
    Ok(pkgs)
}
