#![allow(dead_code)] // This is useful for debugging, so keep it around

use std::{
    fs,
    io,
    path::{
        Path,
        PathBuf,
    },
    process::{
        Command,
        Stdio,
    },
    sync::OnceLock,
};

static WORKSPACE: OnceLock<PathBuf> = OnceLock::new();

pub fn workspace_dir() -> &'static PathBuf {
    WORKSPACE.get_or_init(|| {
        let output = std::process::Command::new(env!("CARGO"))
            .arg("locate-project")
            .arg("--workspace")
            .arg("--message-format=plain")
            .output()
            .unwrap()
            .stdout;
        let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
        cargo_path.parent().unwrap().to_path_buf()
    })
}

pub fn write_and_fmt<P: AsRef<Path>, S: ToString>(path: P, code: S) -> io::Result<()> {
    let path = workspace_dir().join(path);
    fs::write(&path, code.to_string())?;

    // Format but ignore errors
    if let Ok(mut p) = Command::new("rustfmt")
        .arg(&path)
        .stderr(Stdio::null())
        .spawn()
    {
        let _ = p.wait();
    }

    Ok(())
}
