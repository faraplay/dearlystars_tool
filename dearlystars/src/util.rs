use std::io::{Read, Seek, Write};
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    EzError(String),
    BbqParseError(String),
    YamlParseError(String),
    GldParseError(String),
    CsvParseError(Box<dyn std::error::Error>),
    PngEncodingError(png::EncodingError),
    PngDecodingError(png::DecodingError),
    Other(Box<dyn std::error::Error>),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoError(e) => e.fmt(f),
            Error::EzError(e) => e.fmt(f),
            Error::BbqParseError(e) => e.fmt(f),
            Error::YamlParseError(e) => e.fmt(f),
            Error::GldParseError(e) => e.fmt(f),
            Error::CsvParseError(e) => e.fmt(f),
            Error::PngEncodingError(e) => e.fmt(f),
            Error::PngDecodingError(e) => e.fmt(f),
            Error::Other(e) => e.fmt(f),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IoError(value)
    }
}

impl From<binrw::Error> for Error {
    fn from(value: binrw::Error) -> Self {
        match value {
            binrw::Error::Io(io_error) => Error::IoError(io_error),
            _ => Error::Other(value.into()),
        }
    }
}

impl From<png::EncodingError> for Error {
    fn from(value: png::EncodingError) -> Self {
        match value {
            png::EncodingError::IoError(io_error) => Error::IoError(io_error),
            _ => Error::PngEncodingError(value),
        }
    }
}

impl From<png::DecodingError> for Error {
    fn from(value: png::DecodingError) -> Self {
        match value {
            png::DecodingError::IoError(io_error) => Error::IoError(io_error),
            _ => Error::PngDecodingError(value),
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn pad_to_alignment(
    writer: &mut (impl Write + Seek),
    alignment: u32,
    pad_byte: u8,
) -> Result<u32> {
    let current_position = writer.stream_position()? as u32;
    let padded_position = current_position.next_multiple_of(alignment);
    std::io::copy(
        &mut std::io::repeat(pad_byte).take((padded_position - current_position).into()),
        writer,
    )?;
    Ok(padded_position)
}

pub fn mkdir(path: &Path) -> std::io::Result<()> {
    match std::fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}
