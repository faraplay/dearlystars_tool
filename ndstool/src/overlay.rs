use std::{
    io::{Seek, Write},
    path::PathBuf,
};

use binrw::{BinWrite, binrw};

use crate::Result;

#[binrw]
#[brw(little)]
#[repr(C)]
#[derive(Clone)]
pub struct OverlayEntry {
    pub id: u32,
    pub ram_address: u32,
    pub ram_size: u32,
    pub bss_size: u32,
    pub sinit_init: u32,
    pub sinit_init_end: u32,
    pub file_id: u32,
    pub compressed_size_flag: u32,
}

impl OverlayEntry {
    pub fn is_compressed(&self) -> bool {
        self.compressed_size_flag >> 24 == 3
    }
}

pub fn format_overlay_string(overlay_entry: &OverlayEntry) -> String {
    format!("overlay_{:04}.bin", overlay_entry.id)
}

pub fn format_overlay_name(overlay_entry: &OverlayEntry) -> PathBuf {
    PathBuf::from(format_overlay_string(overlay_entry))
}

pub fn write_overlay_table<'a>(
    overlay_entries: impl IntoIterator<Item = &'a OverlayEntry>,
    writer: &mut (impl Write + Seek),
) -> Result<()> {
    for overlay_entry in overlay_entries {
        overlay_entry.write_le(writer)?;
    }
    Ok(())
}
