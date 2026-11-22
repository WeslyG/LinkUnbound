use std::{fs, io, path::Path};

/// Import the old standalone fork before registering the new installer identity.
pub fn import() -> io::Result<()> {
    let destination = linkunbound_core::data_dir();
    let Some(base) = destination.parent() else {
        return Ok(());
    };
    if destination.exists() {
        return Ok(());
    }
    let legacy = base.join("LinkUnbound");
    let installed = base.join("Programs/LinkUnbound/linkunbound-settings.exe");
    let Some(key) = crate::update::fork_public_key() else {
        return Ok(());
    };
    let executable = match fs::read(installed) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if !executable
        .windows(key.len())
        .any(|part| part == key.as_bytes())
        || !legacy.is_dir()
    {
        return Ok(());
    }

    let staging = base.join(format!(
        "LinkUnboundWeslyG-migration-{}",
        std::process::id()
    ));
    fs::create_dir(&staging)?;
    let result =
        copy_settings(&legacy, &staging).and_then(|()| match fs::rename(&staging, &destination) {
            Err(_) if destination.is_dir() => Ok(()),
            result => result,
        });
    if staging.is_dir() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn copy_settings(from: &Path, to: &Path) -> io::Result<()> {
    for name in [
        "preferences.json",
        "rules.json",
        "browsers.json",
        "domain-list-sources.json",
        "shortcut",
        "picked",
    ] {
        let source = from.join(name);
        match fs::symlink_metadata(&source) {
            Ok(info) if info.file_type().is_file() => {
                fs::copy(source, to.join(name))?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    for dir in ["domain-lists", "icons"] {
        let source = from.join(dir);
        let entries = match fs::read_dir(&source) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let destination = to.join(dir);
        fs::create_dir(&destination)?;
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && matches!(
                    entry.path().extension().and_then(|ext| ext.to_str()),
                    Some("txt" | "png" | "ico")
                )
            {
                fs::copy(entry.path(), destination.join(entry.file_name()))?;
            }
        }
    }
    Ok(())
}
