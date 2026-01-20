use std::io::{Read, Seek, SeekFrom};

use binrw::BinRead;

use crate::Result;
use crate::source::SourceTreeNode;

pub struct RomTreeNode {
    name: String,
    pos: u32,
    size: u32,
    children: Option<Vec<RomTreeNode>>,
}

impl RomTreeNode {
    pub fn to_source_tree_node(&self) -> SourceTreeNode {
        SourceTreeNode {
            name: self.name.clone(),
            children: self.children.as_ref().map(|child_nodes| {
                child_nodes
                    .iter()
                    .map(|node| node.to_source_tree_node())
                    .collect()
            }),
        }
    }

    pub fn pos_size(&self) -> (u32, u32) {
        (self.pos, self.size)
    }

    pub fn find_node(&self, node_name: Option<&str>) -> Option<&RomTreeNode> {
        let node_name = match node_name {
            Some(node_name) => node_name,
            None => return Some(self),
        };
        let child_nodes = match &self.children {
            Some(child_nodes) => child_nodes,
            None => return None,
        };
        let (child_name, suffix) = match node_name.split_once("/") {
            Some((child_name, suffix)) => (child_name, Some(suffix)),
            None => (node_name, None),
        };
        let child_node = match child_nodes
            .iter()
            .filter(|node| node.name == child_name)
            .next()
        {
            Some(child_node) => child_node,
            None => return None,
        };
        child_node.find_node(suffix)
    }

    pub fn read_fnt(
        reader: &mut (impl Read + Seek),
        fat_pos_sizes: &[(u32, u32)],
        fnt_offset: u32,
    ) -> Result<RomTreeNode> {
        Self::read_fnt_dir(reader, fat_pos_sizes, fnt_offset, 0xF000, String::new())
    }

    fn read_fnt_dir(
        reader: &mut (impl Read + Seek),
        fat_pos_sizes: &[(u32, u32)],
        fnt_offset: u32,
        dir_id: u16,
        name: String,
    ) -> Result<RomTreeNode> {
        reader.seek(SeekFrom::Start(
            (fnt_offset + 8 * ((dir_id & 0xFFF) as u32)) as u64,
        ))?;
        let entry_start = u32::read_le(reader)?;
        let top_file_id = u16::read_le(reader)?;
        let _parent_id = u16::read_le(reader)?;

        reader.seek(SeekFrom::Start((fnt_offset + entry_start) as u64))?;
        let mut child_nodes = Vec::<RomTreeNode>::new();
        for file_id in top_file_id.. {
            let first_byte = u8::read_le(reader)?;
            let child_name_length = (first_byte & 0x7F) as u32;
            let is_file: bool = (first_byte & 0x80) == 0;
            if child_name_length == 0 {
                break;
            }

            let mut child_name_buffer = vec![0; child_name_length as usize];
            reader.read_exact(&mut child_name_buffer)?;
            let child_name = String::from_utf8(child_name_buffer)?;
            if is_file {
                let (pos, size) = fat_pos_sizes[file_id as usize];
                let child_file = RomTreeNode {
                    name: child_name,
                    pos,
                    size,
                    children: None,
                };
                child_nodes.push(child_file);
            } else {
                let child_dir_id = u16::read_le(reader)?;
                let save_pos = reader.stream_position()?;
                let child_dir = Self::read_fnt_dir(
                    reader,
                    fat_pos_sizes,
                    fnt_offset,
                    child_dir_id,
                    child_name,
                )?;
                child_nodes.push(child_dir);
                reader.seek(SeekFrom::Start(save_pos))?;
            }
        }
        Ok(RomTreeNode {
            name,
            pos: 0,
            size: 0,
            children: Some(child_nodes),
        })
    }
}
