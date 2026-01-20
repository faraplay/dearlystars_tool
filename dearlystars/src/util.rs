use std::io::{Read, Seek, Write};
use std::path::Path;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
