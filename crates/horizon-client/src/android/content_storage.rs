//! App-owned external inboxes. Imports never overwrite an installed file.
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};
const FOLDERS: &[(&str, &str)] = &[
    ("resourcepacks", "resourcepacks"),
    ("mods", "horizon-mods"),
    ("shaderpacks", "shaderpacks"),
    ("controls", "controls"),
];
#[derive(Default, Debug)]
pub struct Report {
    pub imported: usize,
    pub existing: usize,
}

pub fn initialize(game: &Path, external: Option<&Path>) -> io::Result<()> {
    for (public, installed) in FOLDERS {
        fs::create_dir_all(game.join(installed))?;
        if let Some(external) = external {
            fs::create_dir_all(external.join(public))?;
        }
    }
    Ok(())
}
pub fn import(game: &Path, external: &Path) -> io::Result<Report> {
    initialize(game, Some(external))?;
    let mut report = Report::default();
    let mut visited = 0;
    for (public, installed) in FOLDERS {
        copy_missing(
            &external.join(public),
            &game.join(installed),
            0,
            &mut visited,
            &mut report,
        )?;
    }
    Ok(report)
}
fn copy_missing(
    source: &Path,
    destination: &Path,
    depth: usize,
    visited: &mut usize,
    report: &mut Report,
) -> io::Result<()> {
    if depth > 32 {
        return Err(io::Error::other("Pasta de conteúdo excede 32 níveis"));
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        *visited += 1;
        if *visited > 100_000 {
            return Err(io::Error::other("Pasta de conteúdo excede 100000 arquivos"));
        }
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let from = entry.path();
        let to = destination.join(name);
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            // Never follow an installed link out of the game content directory.
            if let Ok(meta) = fs::symlink_metadata(&to) {
                if !meta.is_dir() || meta.file_type().is_symlink() {
                    report.existing += 1;
                    continue;
                }
            }
            fs::create_dir_all(&to)?;
            copy_missing(&from, &to, depth + 1, visited, report)?;
        } else if kind.is_file() {
            if fs::symlink_metadata(&to).is_ok() {
                report.existing += 1;
                continue;
            }
            let temp =
                destination.join(format!(".pojav-import-{}-{}", std::process::id(), visited));
            let result = (|| {
                let mut input = fs::File::open(&from)?.take(1024 * 1024 * 1024 + 1);
                let mut output = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&temp)?;
                let bytes = io::copy(&mut input, &mut output)?;
                if bytes > 1024 * 1024 * 1024 {
                    return Err(io::Error::other("Arquivo de conteúdo excede 1 GiB"));
                }
                output.flush()?;
                output.sync_all()?;
                // A hard link publishes the complete file atomically without replacing a race winner.
                match fs::hard_link(&temp, &to) {
                    Ok(()) => report.imported += 1,
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => report.existing += 1,
                    Err(e) => return Err(e),
                }
                Ok(())
            })();
            let _ = fs::remove_file(&temp);
            result?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_nested_packs_without_overwriting_player_content_or_following_links() {
        let root = std::env::temp_dir().join(format!("pojav-content-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let result = std::panic::catch_unwind(|| {
            let game = root.join("game");
            let external = root.join("external");
            initialize(&game, Some(&external)).unwrap();
            fs::write(external.join("mods/example.hmod"), b"new mod").unwrap();
            fs::create_dir_all(external.join("resourcepacks/Pack/assets")).unwrap();
            fs::write(
                external.join("resourcepacks/Pack/assets/texture.png"),
                b"texture",
            )
            .unwrap();
            fs::write(
                game.join("horizon-mods/example.hmod"),
                b"installed player mod",
            )
            .unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(&game, external.join("mods/escape")).unwrap();
            let report = import(&game, &external).unwrap();
            assert_eq!(report.imported, 1);
            assert_eq!(report.existing, 1);
            assert_eq!(
                fs::read(game.join("horizon-mods/example.hmod")).unwrap(),
                b"installed player mod"
            );
            assert_eq!(
                fs::read(game.join("resourcepacks/Pack/assets/texture.png")).unwrap(),
                b"texture"
            );
            assert!(!game.join("horizon-mods/escape").exists());
            assert_eq!(import(&game, &external).unwrap().imported, 0);
        });
        fs::remove_dir_all(root).unwrap();
        result.unwrap();
    }
}
