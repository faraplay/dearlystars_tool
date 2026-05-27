use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use binrw::BinRead;

use crate::{Result, modcrypt::get_key_ivs};
use crate::{
    error::NdsError,
    header::{DsHeader, DsiExtraFields},
    overlay::OverlayEntry,
    rom_source::rom_tree_node::RomTreeNode,
    source::Source,
};

mod rom_tree_node;
mod source_impl;

#[derive(Clone, Copy)]
pub struct RomFileLocation {
    pub pos: u32,
    pub size: u32,
}

type RomFile = Source<NdsRomFile, DsiRomFile>;

pub struct NdsRomFile {
    reader: File,
    header: DsHeader,
    arm9_has_footer: bool,
    banner_size: u32,
    header_size: u32,
    arm9_overlay_metadatas: Vec<(OverlayEntry, RomFileLocation)>,
    arm7_overlay_metadatas: Vec<(OverlayEntry, RomFileLocation)>,
    root_node: RomTreeNode,
}
pub struct DsiRomFile {
    nds_rom_file: NdsRomFile,
    dsi_fields: DsiExtraFields,
    arm9i_has_footer: bool,
    is_modcrypted: bool,
    key: u128,
    iv1: u128,
    iv2: u128,

    // lazily created
    decrypted_arm9i: Option<Vec<u8>>,
    decrypted_arm7i: Option<Vec<u8>>,
}

impl NdsRomFile {
    fn new(
        reader: File,
        header: DsHeader,
        arm9_has_footer: bool,
        banner_size: u32,
        header_size: u32,
        arm9_overlay_metadatas: Vec<(OverlayEntry, RomFileLocation)>,
        arm7_overlay_metadatas: Vec<(OverlayEntry, RomFileLocation)>,
        root_node: RomTreeNode,
    ) -> NdsRomFile {
        NdsRomFile {
            reader,
            header,
            arm9_has_footer,
            banner_size,
            header_size,
            arm9_overlay_metadatas,
            arm7_overlay_metadatas,
            root_node,
        }
    }
}

impl DsiRomFile {
    fn new(
        nds_rom_file: NdsRomFile,
        dsi_fields: DsiExtraFields,
        arm9i_has_footer: bool,
        is_modcrypted: bool,
        key: u128,
        iv1: u128,
        iv2: u128,
    ) -> DsiRomFile {
        DsiRomFile {
            nds_rom_file,
            dsi_fields,
            arm9i_has_footer,
            is_modcrypted,
            key,
            iv1,
            iv2,
            decrypted_arm9i: None,
            decrypted_arm7i: None,
        }
    }
}

pub fn read_from_rom(mut reader: File) -> Result<RomFile> {
    reader.seek(SeekFrom::Start(0))?;
    let header = DsHeader::read(&mut reader)?;

    let arm9_has_footer = has_footer(&mut reader, header.arm9_rom_offset, header.arm9_size)?;

    let fat_locations = read_fat(
        &mut reader,
        header.fat_offset,
        header.fat_size,
        header.devicecap,
    )?;
    let arm9_overlay_metadatas = get_overlay_metadatas(
        &mut reader,
        header.arm9_overlay_offset,
        header.arm9_overlay_size,
        &fat_locations,
    )?;
    let arm7_overlay_metadatas = get_overlay_metadatas(
        &mut reader,
        header.arm7_overlay_offset,
        header.arm7_overlay_size,
        &fat_locations,
    )?;
    let root_node = RomTreeNode::read_fnt(&mut reader, &fat_locations, header.fnt_offset)?;

    let is_dsi = (header.unitcode & 0x02) != 0;
    if is_dsi {
        reader.seek(SeekFrom::Start(0x180))?;
        let dsi_fields = DsiExtraFields::read(&mut reader)?;
        let arm9i_has_footer = has_footer(
            &mut reader,
            dsi_fields.dsi9_rom_offset,
            dsi_fields.dsi9_size,
        )?;
        let banner_size = dsi_fields.banner_size;
        let header_size: u32 = 0x1000;
        let nds_rom_file = NdsRomFile::new(
            reader,
            header,
            arm9_has_footer,
            banner_size,
            header_size,
            arm9_overlay_metadatas,
            arm7_overlay_metadatas,
            root_node,
        );
        let is_modcrypted = (header.dsi_flags & 0x2) != 0;
        let (key, iv1, iv2) = get_key_ivs(&header, &dsi_fields);
        Ok(RomFile::Dsi(DsiRomFile::new(
            nds_rom_file,
            dsi_fields,
            arm9i_has_footer,
            is_modcrypted,
            key,
            iv1,
            iv2,
        )))
    } else {
        reader.seek(SeekFrom::Start(header.banner_offset.into()))?;
        let version = u16::read_le(&mut reader)?;
        let banner_size: u32 = match version {
            0x0001 => 0x840,
            0x0002 => 0x840 + 0x100,
            0x0003 => 0x840 + 0x200,
            0x0103 => 0x23C0,
            _ => 0x840,
        };
        let header_size: u32 = 0x200;
        let nds_rom_file = NdsRomFile::new(
            reader,
            header,
            arm9_has_footer,
            banner_size,
            header_size,
            arm9_overlay_metadatas,
            arm7_overlay_metadatas,
            root_node,
        );
        Ok(RomFile::Nds(nds_rom_file))
    }
}

fn has_footer(reader: &mut (impl Read + Seek), pos: u32, size: u32) -> Result<bool> {
    reader.seek(SeekFrom::Start((pos + size) as u64))?;
    let nitrocode = u32::read_le(reader)?;
    Ok(nitrocode == 0xDEC00621)
}

fn read_fat(
    reader: &mut (impl Read + Seek),
    fat_offset: u32,
    fat_size: u32,
    devicecap: u8,
) -> Result<Vec<RomFileLocation>> {
    reader.seek(SeekFrom::Start(fat_offset as u64))?;
    let mut locations: Vec<RomFileLocation> = Vec::new();
    for file_id in (0..fat_size).step_by(0x8) {
        let top = u32::read_le(reader)?;
        let bottom = u32::read_le(reader)?;
        let size: u32 = bottom - top;
        if size > (1u32 << (17 + devicecap)) {
            return Err(NdsError {
                message: format!(
                    "File {}: Size is too big. FAT offset {:#X} contains invalid data.",
                    file_id,
                    fat_offset + 8 * file_id
                ),
            }
            .into());
        }
        locations.push(RomFileLocation { pos: top, size });
    }
    Ok(locations)
}

fn get_overlay_metadatas(
    reader: &mut (impl Read + Seek),
    overlay_offset: u32,
    overlay_size: u32,
    fat_locations: &[RomFileLocation],
) -> Result<Vec<(OverlayEntry, RomFileLocation)>> {
    reader.seek(SeekFrom::Start(overlay_offset as u64))?;
    let mut locations: Vec<(OverlayEntry, RomFileLocation)> = Vec::new();
    for _ in (0..overlay_size).step_by(0x20) {
        let overlay_entry = OverlayEntry::read(reader)?;
        let file_id = overlay_entry.id;
        locations.push((overlay_entry, fat_locations[file_id as usize]));
    }
    Ok(locations)
}
