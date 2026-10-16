use std::path::{Path, PathBuf};

pub fn atomic_replace<E>(path: &Path, build: impl FnOnce(&Path) -> Result<(), E>) -> Result<(), E>
where
    E: From<std::io::Error>,
{
    let tmp = tmp_path(path);
    remove_if_exists(&tmp)?;

    if let Err(err) = build(&tmp) {
        let _ = remove_if_exists(&tmp);
        return Err(err);
    }

    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(".tmp");
    path.with_file_name(name)
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_build_keeps_previous_version() {
        let dir = std::env::temp_dir().join(format!("iritsi-fs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data");
        std::fs::write(&path, "old").unwrap();

        let res: Result<(), std::io::Error> = atomic_replace(&path, |tmp| {
            std::fs::write(tmp, "partial")?;
            Err(std::io::Error::other("boom"))
        });
        assert!(res.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
        assert!(!tmp_path(&path).exists());

        atomic_replace::<std::io::Error>(&path, |tmp| std::fs::write(tmp, "new")).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");

        std::fs::remove_dir_all(dir).unwrap();
    }
}
