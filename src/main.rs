mod wad;

use std::fmt::Display;
use std::path::PathBuf;
use std::string::FromUtf8Error;

use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use wad::Wad;

fn main() {
    if let Err(e) = run() {
        println!("A fatal error has occurred: {}", e);
    }
}

fn run() -> Result<(), Error> {
    loop {
        let wad_name = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter a WAD file name")
            .allow_empty(true)
            .interact_text()
            .map_err(Error::Io)?;
        if wad_name.is_empty() {
            break;
        }

        let wad = Wad::from_file(wad_name)?;
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    Io(#[from] std::io::Error),
    Utf8(#[from] FromUtf8Error),
    NotAWad(PathBuf),
    InvalidLumpOrder(i32, String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(i) => write!(f, "I/O error: {}", i),
            Error::NotAWad(p) => write!(f, "not a WAD file: {}", p.to_string_lossy()),
            Error::Utf8(u) => write!(f, "converting from UTF-8 data: {}", u),
            Error::InvalidLumpOrder(w, n) => {
                write!(f, "encountered out-of-order lump at offset {}: '{}'", w, n)
            }
        }
    }
}
