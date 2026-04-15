use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::Result;
use crate::error::NdsError;
use crate::modcrypt::aes_ctr;
use crate::rom_source::{DsiRomFile, NdsRomFile};
use crate::source::{DsiSource, NdsSource, SourceTreeNode};

impl NdsRomFile {
    fn open_pos_size(&mut self, pos: u32, size: u32) -> Result<std::io::Take<&mut File>> {
        let reader = &mut self.reader;
        reader.seek(SeekFrom::Start(pos as u64))?;
        Ok(reader.take(size as u64))
    }
}

impl NdsSource for NdsRomFile {
    fn open_arm9(&mut self) -> Result<impl Read + Seek> {
        self.open_pos_size(
            self.header.arm9_rom_offset,
            self.header.arm9_size + if self.arm9_has_footer { 0xC } else { 0 },
        )
    }
    fn open_arm7(&mut self) -> Result<impl Read + Seek> {
        self.open_pos_size(self.header.arm7_rom_offset, self.header.arm7_size)
    }
    fn open_banner(&mut self) -> Result<impl Read + Seek> {
        self.open_pos_size(self.header.banner_offset, self.banner_size)
    }
    fn open_header(&mut self) -> Result<impl Read + Seek> {
        self.open_pos_size(0, self.header_size)
    }
    fn open_logo(&mut self) -> Result<impl Read + Seek> {
        self.open_pos_size(0xC0, 0x9C)
    }
    fn open_arm9_overlay_table(&mut self) -> Result<impl Read + Seek> {
        self.open_pos_size(
            self.header.arm9_overlay_offset,
            self.header.arm9_overlay_size,
        )
    }
    fn open_arm7_overlay_table(&mut self) -> Result<impl Read + Seek> {
        self.open_pos_size(
            self.header.arm7_overlay_offset,
            self.header.arm7_overlay_size,
        )
    }
    fn arm9_overlay_count(&self) -> u16 {
        self.arm9_overlay_pos_sizes.len() as u16
    }
    fn open_arm9_overlay(&mut self, overlay_index: u16) -> Result<impl Read + Seek> {
        let (pos, size) = self.arm9_overlay_pos_sizes[overlay_index as usize];
        self.open_pos_size(pos, size)
    }
    fn arm7_overlay_count(&self) -> u16 {
        self.arm7_overlay_pos_sizes.len() as u16
    }
    fn open_arm7_overlay(&mut self, overlay_index: u16) -> Result<impl Read + Seek> {
        let (pos, size) = self.arm7_overlay_pos_sizes[overlay_index as usize];
        self.open_pos_size(pos, size)
    }
    fn root_node(&self) -> SourceTreeNode {
        self.root_node.to_source_tree_node()
    }
    fn open_file(&mut self, node_name: &str) -> Result<impl Read + Seek> {
        let (pos, size) = self
            .root_node
            .find_node(Some(node_name))
            .ok_or(NdsError {
                message: String::from("Node not found!"),
            })?
            .pos_size();
        self.open_pos_size(pos, size)
    }
}

impl DsiSource for DsiRomFile {
    fn nds_source(&mut self) -> &mut impl NdsSource {
        &mut self.nds_rom_file
    }
    fn open_arm9i(&mut self) -> Result<impl Read + Seek> {
        if self.decrypted_arm9i.is_none() {
            let pos = self.dsi_fields.dsi9_rom_offset;
            let size = self.dsi_fields.dsi9_size + if self.arm9i_has_footer { 0xC } else { 0 };
            let mut buffer = Vec::new();
            self.nds_rom_file
                .open_pos_size(pos, size)?
                .read_to_end(&mut buffer)?;

            if self.is_modcrypted && (self.dsi_fields.modcrypt1_size != 0) {
                // this assumes that modcrypt1 always encrypts the arm9i binary
                let start =
                    (self.dsi_fields.modcrypt1_start - self.dsi_fields.dsi9_rom_offset) as usize;
                let end = start + self.dsi_fields.modcrypt1_size as usize;
                aes_ctr(&mut buffer[start..end], self.key, self.iv1);
            }
            self.decrypted_arm9i = Some(buffer);
        }
        Ok(Cursor::new(self.decrypted_arm9i.as_ref().unwrap()))
    }
    fn open_arm7i(&mut self) -> Result<impl Read + Seek> {
        if self.decrypted_arm7i.is_none() {
            let pos = self.dsi_fields.dsi7_rom_offset;
            let size = self.dsi_fields.dsi7_size;
            let mut buffer = Vec::new();
            self.nds_rom_file
                .open_pos_size(pos, size)?
                .read_to_end(&mut buffer)?;

            if self.is_modcrypted && (self.dsi_fields.modcrypt2_size != 0) {
                // this assumes that modcrypt2 always encrypts the arm7i binary
                let start =
                    (self.dsi_fields.modcrypt2_start - self.dsi_fields.dsi7_rom_offset) as usize;
                let end = start + self.dsi_fields.modcrypt2_size as usize;
                aes_ctr(&mut buffer[start..end], self.key, self.iv2);
            }
            self.decrypted_arm7i = Some(buffer);
        }
        Ok(Cursor::new(self.decrypted_arm7i.as_ref().unwrap()))
    }
}
