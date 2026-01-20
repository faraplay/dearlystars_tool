use std::io::{Read, Seek, SeekFrom, Write};

use binrw::{BinRead, BinWrite};

use crate::Result;
use crate::util::{pad_to_alignment, pad_to_position};
use crate::{
    header::{DsHeader, DsiExtraFields},
    source::{DsiSource, NdsSource, Source, SourceTreeNode},
    write_rom::indexed_tree_node::IndexedTreeNode,
};

mod indexed_tree_node;

const NINTENDO_LOGO: [u8; 0x9C] = [
    0x24, 0xFF, 0xAE, 0x51, 0x69, 0x9A, 0xA2, 0x21, 0x3D, 0x84, 0x82, 0x0A, 0x84, 0xE4, 0x09, 0xAD,
    0x11, 0x24, 0x8B, 0x98, 0xC0, 0x81, 0x7F, 0x21, 0xA3, 0x52, 0xBE, 0x19, 0x93, 0x09, 0xCE, 0x20,
    0x10, 0x46, 0x4A, 0x4A, 0xF8, 0x27, 0x31, 0xEC, 0x58, 0xC7, 0xE8, 0x33, 0x82, 0xE3, 0xCE, 0xBF,
    0x85, 0xF4, 0xDF, 0x94, 0xCE, 0x4B, 0x09, 0xC1, 0x94, 0x56, 0x8A, 0xC0, 0x13, 0x72, 0xA7, 0xFC,
    0x9F, 0x84, 0x4D, 0x73, 0xA3, 0xCA, 0x9A, 0x61, 0x58, 0x97, 0xA3, 0x27, 0xFC, 0x03, 0x98, 0x76,
    0x23, 0x1D, 0xC7, 0x61, 0x03, 0x04, 0xAE, 0x56, 0xBF, 0x38, 0x84, 0x00, 0x40, 0xA7, 0x0E, 0xFD,
    0xFF, 0x52, 0xFE, 0x03, 0x6F, 0x95, 0x30, 0xF1, 0x97, 0xFB, 0xC0, 0x85, 0x60, 0xD6, 0x80, 0x25,
    0xA9, 0x63, 0xBE, 0x03, 0x01, 0x4E, 0x38, 0xE2, 0xF9, 0xA2, 0x34, 0xFF, 0xBB, 0x3E, 0x03, 0x44,
    0x78, 0x00, 0x90, 0xCB, 0x88, 0x11, 0x3A, 0x94, 0x65, 0xC0, 0x7C, 0x63, 0x87, 0xF0, 0x3C, 0xAF,
    0xD6, 0x25, 0xE4, 0x8B, 0x38, 0x0A, 0xAC, 0x72, 0x21, 0xD4, 0xF8, 0x07,
];

const ARM9_ALIGNMENT: u32 = 0x200;
const ARM7_ALIGNMENT: u32 = 0x200;
const ARM7_MIN: u32 = 0x8000;
const FNT_ALIGNMENT: u32 = 0x200;
const FAT_ALIGNMENT: u32 = 0x200;
const BANNER_ALIGNMENT: u32 = 0x200;
const FILE_ALIGNMENT: u32 = 0x200;
const SECTOR_ALIGNMENT: u32 = 0x400;

pub fn write_to_rom<T: NdsSource, U: DsiSource>(
    source: &mut Source<T, U>,
    writer: &mut (impl Write + Seek),
) -> Result<()> {
    match source {
        Source::Nds(nds_source) => write_nds_rom(nds_source, writer),
        Source::Dsi(dsi_source) => write_dsi_rom(dsi_source, writer),
    }
}

fn write_nds_rom(source: &mut impl NdsSource, writer: &mut (impl Write + Seek)) -> Result<()> {
    let mut header = DsHeader::read(&mut source.open_header()?)?;

    header.rom_header_size = 0x200;

    write_rom_common(source, writer, &mut header, false)?;

    writer.seek(SeekFrom::Start(0))?;
    header.write(writer)?;
    Ok(())
}

fn write_dsi_rom(source: &mut impl DsiSource, writer: &mut (impl Write + Seek)) -> Result<()> {
    let mut header_reader = source.nds_source().open_header()?;
    let mut header = DsHeader::read(&mut header_reader)?;
    let mut dsi_header = DsiExtraFields::read(&mut header_reader)?;
    drop(header_reader);

    header.rom_header_size = 0x4000;

    write_rom_common(source.nds_source(), writer, &mut header, true)?;

    // DSi sections
    dsi_header.banner_size = source.nds_source().open_banner()?.seek(SeekFrom::End(0))? as u32;

    // DSi ARM9 binary
    {
        dsi_header.dsi9_rom_offset = pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
        let size = std::io::copy(&mut source.open_arm9i()?, writer)? as u32;
        dsi_header.dsi9_size = size.next_multiple_of(4);
    }

    // DSi ARM7 binary
    {
        dsi_header.dsi7_rom_offset = pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
        let size = std::io::copy(&mut source.open_arm7i()?, writer)? as u32;
        dsi_header.dsi7_size = size.next_multiple_of(4);
    }

    // fix up header CRCs and write header

    writer.seek(SeekFrom::Start(0))?;
    header.write(writer)?;
    dsi_header.write(writer)?;
    Ok(())
}

fn write_rom_common(
    source: &mut impl NdsSource,
    writer: &mut (impl Write + Seek),
    header: &mut DsHeader,
    b_secure_syscalls: bool,
) -> Result<()> {
    // load a logo
    // use Nintendo logo
    header.logo = NINTENDO_LOGO;

    writer.seek(SeekFrom::Start(header.rom_header_size.into()))?;

    // ARM9 binary
    {
        header.arm9_rom_offset = pad_to_alignment(writer, ARM9_ALIGNMENT, 0xFF)?;
        let (entry_address, ram_address) =
            match (header.arm9_entry_address, header.arm9_ram_address) {
                (0, 0) => (0x02000000, 0x02000000),
                (0, value) => (value, value),
                (value, 0) => (value, value),
                (value1, value2) => (value1, value2),
            };

        // add dummy area for secure syscalls
        header.arm9_size = 0;
        if b_secure_syscalls {
            let mut arm9_reader = source.open_arm9()?;
            let x = u32::read_le(&mut arm9_reader)?;
            if x != 0xE7FFDEFF {
                for _ in (0..0x800).step_by(4) {
                    0xE7FFDEFFu32.write_le(writer)?;
                }
                header.arm9_size = 0x800;
            }
        }

        let size = copy_get_size_without_footer(&mut source.open_arm9()?, writer)?;
        header.arm9_entry_address = entry_address;
        header.arm9_ram_address = ram_address;
        header.arm9_size += size.next_multiple_of(4);

        if header.rom_header_size > 0x200
            && (entry_address - ram_address) == 0x800
            && header.arm9_size < 0x4000
        {
            // Pad the arm9 binary to 16kb
            let needed_padding: u32 = 0x4000 - header.arm9_size;
            header.arm9_size = 0x4000;
            writer.seek(SeekFrom::Current((needed_padding - 1).into()))?;
            0u8.write_le(writer)?;
        }
    }

    // ARM9 overlay table
    header.arm9_overlay_offset = pad_to_alignment(writer, FILE_ALIGNMENT, 0xFF)?;
    let size = std::io::copy(&mut source.open_arm9_overlay_table()?, writer)? as u32;
    header.arm9_overlay_size = size;
    let arm9_overlay_files = (size / 0x20) as u16;
    if size == 0 {
        header.arm9_overlay_offset = 0;
    }

    // manually added ARM9 overlay files. each file is padded with 0xFF's
    let mut overlay_fat_entries = Vec::<(u32, u32)>::new();
    for overlay_index in 0..arm9_overlay_files {
        let fat_entry = add_file(&mut source.open_arm9_overlay(overlay_index)?, writer)?;
        overlay_fat_entries.push(fat_entry);
    }

    // ARM7 binary
    header.arm7_rom_offset = std::cmp::max(
        (writer.stream_position()? as u32).next_multiple_of(ARM7_ALIGNMENT),
        ARM7_MIN,
    );
    pad_to_position(writer, header.arm7_rom_offset, 0xFF)?;
    {
        let (entry_address, ram_address) =
            match (header.arm7_entry_address, header.arm7_ram_address) {
                (0, 0) => (0x037F8000, 0x037F8000),
                (0, value) => (value, value),
                (value, 0) => (value, value),
                (value1, value2) => (value1, value2),
            };

        let size = std::io::copy(&mut source.open_arm7()?, writer)? as u32;
        header.arm7_entry_address = entry_address;
        header.arm7_ram_address = ram_address;
        header.arm7_size = size.next_multiple_of(4);
    }

    // ARM7 overlay table
    header.arm7_overlay_offset = pad_to_alignment(writer, ARM7_ALIGNMENT, 0xFF)?;
    let size = std::io::copy(&mut source.open_arm7_overlay_table()?, writer)? as u32;
    header.arm7_overlay_size = size;
    let arm7_overlay_files = (size / 0x20) as u16;
    if size == 0 {
        header.arm7_overlay_offset = 0;
    }

    // manually added ARM7 overlay files, just like for ARM9
    for overlay_index in 0..arm7_overlay_files {
        let fat_entry = add_file(&mut source.open_arm7_overlay(overlay_index)?, writer)?;
        overlay_fat_entries.push(fat_entry);
    }

    let overlay_files = arm9_overlay_files + arm7_overlay_files;

    // filesystem
    {
        // read directory structure
        let root_node = &source.root_node();

        // calculate offsets required for FNT and FAT
        header.fnt_offset = (writer.stream_position()? as u32).next_multiple_of(FNT_ALIGNMENT);
        writer.seek(SeekFrom::Start(header.fnt_offset as u64))?;
        let (fnt_size, fat_size) = add_name_table(writer, root_node, overlay_files)?;

        header.fnt_size = fnt_size;
        header.fat_offset = (header.fnt_offset + header.fnt_size).next_multiple_of(FAT_ALIGNMENT);
        header.fat_size = fat_size;

        // banner after FNT/FAT
        header.banner_offset =
            (header.fat_offset + header.fat_size).next_multiple_of(BANNER_ALIGNMENT);
        pad_to_position(writer, header.banner_offset, 0xFF)?;
        std::io::copy(&mut source.open_banner()?, writer)? as u32;

        let file_fat_entries = add_files(source, writer, root_node)?;
        writer.seek(SeekFrom::Start(header.fat_offset as u64))?;
        overlay_fat_entries.write_le(writer)?;
        file_fat_entries.write_le(writer)?;

        writer.seek(SeekFrom::End(0))?;
    }

    // align file size
    let mut newfilesize: u32 = (writer.stream_position()? as u32).next_multiple_of(4);
    header.application_end_offset = newfilesize;
    pad_to_position(writer, newfilesize, 0x00)?;

    // calculate device capacity
    newfilesize = newfilesize.next_power_of_two();
    if newfilesize <= 128 * 1024 {
        newfilesize = 128 * 1024;
    }
    header.devicecap = ((newfilesize.ilog2() as i32) - 17) as u8;

    Ok(())
}


fn copy_get_size_without_footer(
    reader: &mut (impl Read + Seek),
    writer: &mut (impl Write + Seek),
) -> Result<u32> {
    let size = std::io::copy(reader, writer)? as u32;
    reader.seek(SeekFrom::End(-3 * 4))?;
    let nitrocode: u32 = u32::read_le(reader)?;
    let size_without_footer = if nitrocode == 0xDEC00621 {
        size - 3 * 4
    } else {
        size
    };
    Ok(size_without_footer)
}

fn add_file(
    reader: &mut (impl Read + Seek),
    writer: &mut (impl Write + Seek),
) -> Result<(u32, u32)> {
    let top: u32 = pad_to_alignment(writer, FILE_ALIGNMENT, 0xFF)?;
    let size: u32 = std::io::copy(reader, writer)? as u32;
    Ok((top, top + size))
}

fn add_name_table(
    writer: &mut (impl Write + Seek),
    root_node: &SourceTreeNode,
    first_file_id: u16,
) -> Result<(u32, u32)> {
    let mut free_dir_id: u16 = 0xF000;
    let indexed_root_node = IndexedTreeNode::from_source_tree_node(root_node, &mut free_dir_id);
    let mut free_file_id: u16 = first_file_id;
    let (dir_data, name_data) = indexed_root_node.name_table(&mut free_file_id);
    dir_data.write_le(writer)?;
    name_data.write_le(writer)?;
    let fnt_size = (8 * dir_data.len() + name_data.len()) as u32;
    let fat_size = 8 * (free_file_id as u32); // top offset (4), bottom offset (4)
    Ok((fnt_size, fat_size))
}

fn add_files(
    source: &mut impl NdsSource,
    writer: &mut (impl Write + Seek),
    root_node: &SourceTreeNode,
) -> Result<Vec<(u32, u32)>> {
    add_dir_tree_node(source, writer, None, root_node)
}

fn add_dir_tree_node(
    source: &mut impl NdsSource,
    writer: &mut (impl Write + Seek),
    node_path: Option<&str>,
    node: &SourceTreeNode,
) -> Result<Vec<(u32, u32)>> {
    let child_nodes = node.children.as_ref().unwrap();
    let mut fat_entries = Vec::<(u32, u32)>::new();
    // Iterate through files that are direct children first
    for child_node in child_nodes {
        if child_node.children.is_some() {
            continue;
        }
        let child_node_name: &str = match node_path {
            Some(node_name) => &format!("{}/{}", node_name, child_node.name),
            None => &child_node.name,
        };
        println!("{}", child_node_name);
        fat_entries.push(add_file(&mut source.open_file(child_node_name)?, writer)?);
    }
    // Then iterate through subdirectories
    for child_node in child_nodes {
        if child_node.children.is_none() {
            continue;
        }
        let child_node_name: &str = match node_path {
            Some(node_name) => &format!("{}/{}", node_name, child_node.name),
            None => &child_node.name,
        };
        fat_entries
            .extend(add_dir_tree_node(source, writer, Some(child_node_name), child_node)?.iter());
    }
    Ok(fat_entries)
}
