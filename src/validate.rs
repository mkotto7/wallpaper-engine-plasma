use std::path::{Path, PathBuf};
use clap::Error;
use clap::error::ErrorKind;

const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];

pub fn validate_dir(dir: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(dir);
    if !path.exists() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} does not exist", path.display()),
        ));
    }

    if !path.is_dir() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} is not a directory", path.display()),
        ));
    }

    Ok(path)
}

pub fn validate_file(file: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(file);

    if !path.exists() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} does not exist", path.display()),
        ));
    }

    if !path.is_file() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} is not a file", path.display()),
        ));
    }

    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            Error::raw(
                ErrorKind::ValueValidation,
                "could not read extension".to_string(),
            )
        })?;

    if !EXTENSIONS.contains(&ext) {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            "file is not supported",
        ));
    }

    Ok(path)
}

pub fn is_valid_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}
