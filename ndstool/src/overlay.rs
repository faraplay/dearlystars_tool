use std::path::PathBuf;

use binrw::binrw;

#[binrw]
#[brw(little)]
#[repr(C)]
pub struct OverlayEntry {
    pub id: u32,
    pub ram_address: u32,
    pub ram_size: u32,
    pub bss_size: u32,
    pub sinit_init: u32,
    pub sinit_init_end: u32,
    pub file_id: u32,
    pub reserved: u32,
}

pub fn format_overlay_string(overlay_id: u16) -> String {
    format!("overlay_{overlay_id:04}.bin")
}

pub fn format_overlay_name(overlay_id: u16) -> PathBuf {
    PathBuf::from(format_overlay_string(overlay_id))
}
