use std::io::{Read, Seek, SeekFrom, Write};

use binrw::{BinRead, BinWrite};

use crate::Result;
use crate::digest::sha1_hmac;
use crate::key_encryption::{decrypt_arm9, encrypt_arm9};
use crate::modcrypt::modcrypt;
use crate::util::{pad_n, pad_to_alignment, pad_to_position};
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
const BLOCK_SECTORCOUNT: u32 = 0x20;
const DSI_ALIGNMENT: u32 = 0x100000;

pub fn write_to_rom<T: NdsSource, U: DsiSource>(
    source: &mut Source<T, U>,
    writer: &mut (impl Read + Write + Seek),
) -> Result<()> {
    match source {
        Source::Nds(nds_source) => write_nds_rom(nds_source, writer),
        Source::Dsi(dsi_source) => write_dsi_rom(dsi_source, writer),
    }
}

fn write_nds_rom(
    source: &mut impl NdsSource,
    writer: &mut (impl Read + Write + Seek),
) -> Result<()> {
    let mut header = DsHeader::read(&mut source.open_header()?)?;

    header.rom_header_size = 0x200;

    // load a logo
    // use Nintendo logo
    header.logo = NINTENDO_LOGO;

    writer.seek(SeekFrom::Start(header.rom_header_size.into()))?;

    let arm9_overlay_fat_entries = write_arm9(source, writer, &mut header, false)?;
    let arm7_overlay_fat_entries = write_arm7(source, writer, &mut header)?;
    let overlay_fat_entries = vec![arm9_overlay_fat_entries, arm7_overlay_fat_entries].concat();
    write_filesystem(source, writer, &mut header, &overlay_fat_entries)?;

    set_file_size(writer, &mut header)?;

    writer.seek(SeekFrom::Start(0))?;
    header.write(writer)?;
    Ok(())
}

fn write_dsi_rom(
    source: &mut impl DsiSource,
    writer: &mut (impl Read + Write + Seek),
) -> Result<()> {
    let mut header_reader = source.nds_source().open_header()?;
    let mut header = DsHeader::read(&mut header_reader)?;
    let mut dsi_header = DsiExtraFields::read(&mut header_reader)?;
    drop(header_reader);

    header.rom_header_size = 0x4000;

    // load a logo
    // use Nintendo logo
    header.logo = NINTENDO_LOGO;

    writer.seek(SeekFrom::Start(header.rom_header_size.into()))?;

    let arm9_overlay_fat_entries = write_arm9(source.nds_source(), writer, &mut header, true)?;
    let arm7_overlay_fat_entries = write_arm7(source.nds_source(), writer, &mut header)?;
    let overlay_fat_entries = vec![arm9_overlay_fat_entries, arm7_overlay_fat_entries].concat();
    write_filesystem(
        source.nds_source(),
        writer,
        &mut header,
        &overlay_fat_entries,
    )?;

    reserve_digests_space(source, writer, &mut dsi_header)?;

    set_file_size(writer, &mut header)?;

    // DSi sections

    // fill 0x3000 bytes with some data (not sure what this is)
    let mut junk_data = [0u8; 0x1000];
    writer.seek(SeekFrom::Start(0x8000))?;
    writer.read_exact(&mut junk_data)?;
    writer.seek(SeekFrom::End(0))?;
    pad_to_alignment(writer, DSI_ALIGNMENT, 0xFF)?;
    for _ in 0..3 {
        writer.write_all(&junk_data)?;
    }

    // DSi ARM9 binary
    dsi_header.dsi9_rom_offset = pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
    let size = std::io::copy(&mut source.open_arm9i()?, writer)? as u32;
    dsi_header.dsi9_size = size.next_multiple_of(4);
    // DSi ARM7 binary
    dsi_header.dsi7_rom_offset = pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
    let size = std::io::copy(&mut source.open_arm7i()?, writer)? as u32;
    dsi_header.dsi7_size = size.next_multiple_of(4);

    // digest_twl_start is not set in reserve_digests_space, so we set it here
    dsi_header.digest_twl_start = dsi_header.dsi9_rom_offset;

    // pad out rest of file with 0xFF
    let newfilesize = pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
    dsi_header.total_rom_size = newfilesize;
    pad_to_position(writer, 1 << (header.devicecap + 17), 0xFF)?;

    // set dsi header fields
    set_dsi_header_fields(source, &mut header, &mut dsi_header)?;

    // write digests and hashes
    write_digests(writer, &dsi_header)?;
    write_hashes(writer, header, &mut dsi_header)?;

    // decrypt secure area
    decrypt_secure_area(writer, &mut header)?;
    // encrypt modcrypted areas
    modcrypt(writer, &header, &dsi_header)?;

    writer.seek(SeekFrom::Start(0))?;
    header.write(writer)?;
    dsi_header.write(writer)?;
    Ok(())
}

fn write_arm9(
    source: &mut impl NdsSource,
    writer: &mut (impl Read + Write + Seek),
    header: &mut DsHeader,
    b_secure_syscalls: bool,
) -> Result<Vec<(u32, u32)>> {
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

    // encrypt the secure area (if arm9 is decrypted)
    encrypt_secure_area(writer, header)?;

    // ARM9 overlay table
    header.arm9_overlay_offset = pad_to_alignment(writer, FILE_ALIGNMENT, 0xFF)?;
    let size = std::io::copy(&mut source.open_arm9_overlay_table()?, writer)? as u32;
    header.arm9_overlay_size = size;
    let arm9_overlay_files = (size / 0x20) as u16;
    if size == 0 {
        header.arm9_overlay_offset = 0;
    }

    // manually added ARM9 overlay files. each file is padded with 0xFF's
    let mut arm9_overlay_fat_entries = Vec::<(u32, u32)>::new();
    for overlay_index in 0..arm9_overlay_files {
        let fat_entry = add_file(&mut source.open_arm9_overlay(overlay_index)?, writer)?;
        arm9_overlay_fat_entries.push(fat_entry);
    }

    Ok(arm9_overlay_fat_entries)
}

fn write_arm7(
    source: &mut impl NdsSource,
    writer: &mut (impl Read + Write + Seek),
    header: &mut DsHeader,
) -> Result<Vec<(u32, u32)>> {
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
    let mut arm7_overlay_fat_entries = Vec::<(u32, u32)>::new();
    for overlay_index in 0..arm7_overlay_files {
        let fat_entry = add_file(&mut source.open_arm7_overlay(overlay_index)?, writer)?;
        arm7_overlay_fat_entries.push(fat_entry);
    }

    Ok(arm7_overlay_fat_entries)
}

fn write_filesystem(
    source: &mut impl NdsSource,
    writer: &mut (impl Read + Write + Seek),
    header: &mut DsHeader,
    overlay_fat_entries: &[(u32, u32)],
) -> Result<()> {
    // read directory structure
    let root_node = &source.root_node();

    // calculate offsets required for FNT and FAT
    header.fnt_offset = (writer.stream_position()? as u32).next_multiple_of(FNT_ALIGNMENT);
    writer.seek(SeekFrom::Start(header.fnt_offset as u64))?;
    let (fnt_size, fat_size) = add_name_table(writer, root_node, overlay_fat_entries.len() as u16)?;

    header.fnt_size = fnt_size;
    header.fat_offset = (header.fnt_offset + header.fnt_size).next_multiple_of(FAT_ALIGNMENT);
    header.fat_size = fat_size;

    // banner after FNT/FAT
    header.banner_offset = (header.fat_offset + header.fat_size).next_multiple_of(BANNER_ALIGNMENT);
    pad_to_position(writer, header.banner_offset, 0xFF)?;
    std::io::copy(&mut source.open_banner()?, writer)? as u32;

    let file_fat_entries = add_files(source, writer, root_node)?;
    writer.seek(SeekFrom::Start(header.fat_offset as u64))?;
    overlay_fat_entries.write_le(writer)?;
    file_fat_entries.write_le(writer)?;

    writer.seek(SeekFrom::End(0))?;
    Ok(())
}

fn encrypt_secure_area(
    writer: &mut (impl Read + Write + Seek),
    header: &mut DsHeader,
) -> Result<()> {
    // encrypt secure area if decrypted
    writer.seek(SeekFrom::Start(0x4000))?;
    let magic = u64::read_le(writer)?;
    if magic == 0xE7FFDEFFE7FFDEFF {
        eprintln!("Encrypting secure area...");
        let mut data_to_encrypt = [0u8; 0x4000];
        writer.seek(SeekFrom::Start(0x4000))?;
        writer.read_exact(&mut data_to_encrypt)?;

        let gamecode = u32::from_le_bytes(header.gamecode);
        let encrypted_data = encrypt_arm9(gamecode, &data_to_encrypt);

        writer.seek(SeekFrom::Start(0x4000))?;
        writer.write_all(&encrypted_data)?;
        writer.seek(SeekFrom::End(0))?;
    }
    Ok(())
}

fn decrypt_secure_area(
    writer: &mut (impl Read + Write + Seek),
    header: &mut DsHeader,
) -> Result<()> {
    eprintln!("Decrypting secure area...");
    let mut data_to_decrypt = [0u8; 0x4000];
    writer.seek(SeekFrom::Start(0x4000))?;
    writer.read_exact(&mut data_to_decrypt)?;

    let gamecode = u32::from_le_bytes(header.gamecode);
    let decrypted_data = decrypt_arm9(gamecode, &data_to_decrypt);

    writer.seek(SeekFrom::Start(0x4000))?;
    writer.write_all(&decrypted_data)?;
    writer.seek(SeekFrom::End(0))?;
    Ok(())
}

fn set_dsi_header_fields(
    source: &mut impl DsiSource,
    header: &mut DsHeader,
    dsi_header: &mut DsiExtraFields,
) -> Result<()> {
    // Values of these should be read from header file
    // header.dsi_flags = 0x01;
    // header.rom_control_info3 = 0x051E;

    let ntr_region_size = (header
        .application_end_offset
        .next_multiple_of(DSI_ALIGNMENT)
        >> 19) as u16;
    header.dsi_ntr_rom_region_end = ntr_region_size;
    header.dsi_twl_rom_region_start = ntr_region_size;

    dsi_header.global_mbk_setting = [
        0x81, 0x85, 0x89, 0x8D, 0x80, 0x84, 0x88, 0x8C, 0x90, 0x94, 0x98, 0x9C, 0x80, 0x84, 0x88,
        0x8C, 0x90, 0x94, 0x98, 0x9C,
    ];
    dsi_header.arm9_mbk_setting = [0x00000000, 0x07C03740, 0x07403700];
    dsi_header.arm7_mbk_setting = [
        if (header.unitcode & 1) != 0 {
            0x080037C0
        } else {
            0x00403000
        },
        0x07C03740,
        0x07403700,
    ];
    dsi_header.mbk9_wramcnt_setting = 0x0300000F;

    // Values of these should be read from header file or calculated
    // dsi_header.region_flags = 0xFFFFFFFF;
    // dsi_header.access_control = 0x00000138;
    // dsi_header.scfg_ext_mask = 0x80040407;
    // dsi_header.appflags = 0x01;

    // dsi_header.device_list_ram_address = 0x03800000;

    dsi_header.banner_size = source.nds_source().open_banner()?.seek(SeekFrom::End(0))? as u32;
    // dsi_header.shared_20000_size = 0x00;
    // dsi_header.shared_20001_size = 0x00;
    // dsi_header.eula_version = 0x01;
    // dsi_header.use_ratings = 0x00;

    dsi_header.tid_low = (header.gamecode[3] as u32)
        | (header.gamecode[2] as u32) << 8
        | (header.gamecode[1] as u32) << 16
        | (header.gamecode[0] as u32) << 24;
    dsi_header.tid_high = 0x00030000;

    Ok(())
}

const HASH_SIZE: usize = 20;

fn reserve_digests_space(
    source: &mut impl DsiSource,
    writer: &mut (impl Read + Write + Seek),
    dsi_header: &mut DsiExtraFields,
) -> Result<()> {
    pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
    dsi_header.digest_sector_size = SECTOR_ALIGNMENT;
    dsi_header.digest_block_sectorcount = BLOCK_SECTORCOUNT;

    let digest_ntr_end = writer.stream_position()? as u32;
    dsi_header.digest_ntr_start = 0x4000;
    dsi_header.digest_ntr_size = digest_ntr_end - dsi_header.digest_ntr_start;
    let arm9i_size =
        (source.open_arm9i()?.seek(SeekFrom::End(0))? as u32).next_multiple_of(SECTOR_ALIGNMENT);
    let arm7i_size =
        (source.open_arm7i()?.seek(SeekFrom::End(0))? as u32).next_multiple_of(SECTOR_ALIGNMENT);
    dsi_header.digest_twl_size = arm9i_size + arm7i_size;

    // reserve space for sector hashes
    dsi_header.sector_hashtable_start = pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
    let sectors_count =
        (dsi_header.digest_ntr_size + dsi_header.digest_twl_size) / dsi_header.digest_sector_size;
    let sectors_count_padded = sectors_count.next_multiple_of(dsi_header.digest_block_sectorcount);
    dsi_header.sector_hashtable_size = sectors_count_padded * (HASH_SIZE as u32);
    pad_n(writer, dsi_header.sector_hashtable_size, 0x00)?;

    // reserve space for block hashes
    dsi_header.block_hashtable_start = pad_to_alignment(writer, SECTOR_ALIGNMENT, 0xFF)?;
    dsi_header.block_hashtable_size =
        dsi_header.sector_hashtable_size / dsi_header.digest_block_sectorcount;
    pad_n(writer, dsi_header.block_hashtable_size, 0x00)?;

    pad_to_alignment(writer, FILE_ALIGNMENT, 0xFF)?;
    Ok(())
}

fn write_digests(
    writer: &mut (impl Read + Write + Seek),
    dsi_header: &DsiExtraFields,
) -> Result<()> {
    let digest_sector_size = dsi_header.digest_sector_size;
    let block_sectorcount = dsi_header.digest_block_sectorcount;

    // sector digests
    eprintln!("Calculating sector digests...");
    let mut sector_hashes = Vec::<u8>::new();
    for position in (dsi_header.digest_ntr_start
        ..dsi_header.digest_ntr_start + dsi_header.digest_ntr_size)
        .step_by(digest_sector_size as usize)
    {
        sector_hashes.extend(&sha1_hmac(writer, position, digest_sector_size)?);
    }
    for position in (dsi_header.digest_twl_start
        ..dsi_header.digest_twl_start + dsi_header.digest_twl_size)
        .step_by(digest_sector_size as usize)
    {
        sector_hashes.extend(&sha1_hmac(writer, position, digest_sector_size)?);
    }
    writer.seek(SeekFrom::Start(dsi_header.sector_hashtable_start as u64))?;
    writer.write_all(&sector_hashes)?;

    // block digests
    eprintln!("Calculating block digests...");
    let mut block_hashes = Vec::<u8>::new();
    let block_size = block_sectorcount * HASH_SIZE as u32;
    for position in (dsi_header.sector_hashtable_start
        ..dsi_header.sector_hashtable_start + dsi_header.sector_hashtable_size)
        .step_by(block_size as usize)
    {
        block_hashes.extend(&sha1_hmac(writer, position, block_size)?);
    }
    writer.seek(SeekFrom::Start(dsi_header.block_hashtable_start as u64))?;
    writer.write_all(&block_hashes)?;

    Ok(())
}

fn write_hashes(
    writer: &mut (impl Read + Write + Seek),
    header: DsHeader,
    dsi_header: &mut DsiExtraFields,
) -> Result<()> {
    dsi_header.hmac_arm9 = sha1_hmac(writer, header.arm9_rom_offset, header.arm9_size)?;
    dsi_header.hmac_arm7 = sha1_hmac(writer, header.arm7_rom_offset, header.arm7_size)?;
    dsi_header.hmac_digest_master = sha1_hmac(
        writer,
        dsi_header.block_hashtable_start,
        dsi_header.block_hashtable_size,
    )?;
    dsi_header.hmac_icon_title = sha1_hmac(writer, header.banner_offset, dsi_header.banner_size)?;
    dsi_header.hmac_arm9i = sha1_hmac(writer, dsi_header.dsi9_rom_offset, dsi_header.dsi9_size)?;
    dsi_header.hmac_arm7i = sha1_hmac(writer, dsi_header.dsi7_rom_offset, dsi_header.dsi7_size)?;
    dsi_header.hmac_arm9_no_secure = sha1_hmac(
        writer,
        0x8000,
        header.arm9_size - (0x8000 - header.arm9_rom_offset),
    )?;
    dsi_header.rsa_signature = [0xAA; 0x80];
    Ok(())
}

fn set_file_size(writer: &mut (impl Read + Write + Seek), header: &mut DsHeader) -> Result<()> {
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
        eprintln!("{}", child_node_name);
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
