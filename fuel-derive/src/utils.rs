use std::{
    fs,
    io,
    path::Path,
    process::Command,
};

pub fn write_and_fmt<P: AsRef<Path>, S: ToString>(path: P, code: S) -> io::Result<()> {
    fs::write(&path, code.to_string())?;

    Command::new("rustfmt").arg(path.as_ref()).spawn()?.wait()?;

    Ok(())
}
