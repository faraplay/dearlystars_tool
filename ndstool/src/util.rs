use std::io::{Read, Seek, Write};
use std::path::Path;

use crate::Result;

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

pub fn pad_to_position(
    writer: &mut (impl Write + Seek),
    position: u32,
    pad_byte: u8,
) -> Result<()> {
    let current_position = writer.stream_position()? as u32;
    std::io::copy(
        &mut std::io::repeat(pad_byte).take((position - current_position).into()),
        writer,
    )?;
    Ok(())
}

pub fn pad_n(writer: &mut (impl Write + Seek), n: u32, pad_byte: u8) -> Result<()> {
    std::io::copy(&mut std::io::repeat(pad_byte).take(n as u64), writer)?;
    Ok(())
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
