use std::io::{Read, Seek};

use crate::Result;

#[derive(Debug, Clone)]
pub struct SourceTreeNode {
    pub name: String,
    pub children: Option<Vec<SourceTreeNode>>,
}

pub trait NdsSource {
    fn open_arm9(&mut self) -> Result<impl Read + Seek>;
    fn open_arm7(&mut self) -> Result<impl Read + Seek>;
    fn open_banner(&mut self) -> Result<impl Read + Seek>;
    fn open_header(&mut self) -> Result<impl Read + Seek>;
    fn open_logo(&mut self) -> Result<impl Read + Seek>;
    fn open_arm9_overlay_table(&mut self) -> Result<impl Read + Seek>;
    fn open_arm7_overlay_table(&mut self) -> Result<impl Read + Seek>;
    fn arm9_overlay_count(&self) -> u16;
    fn open_arm9_overlay(&mut self, overlay_index: u16) -> Result<impl Read + Seek>;
    fn arm7_overlay_count(&self) -> u16;
    fn open_arm7_overlay(&mut self, overlay_index: u16) -> Result<impl Read + Seek>;
    fn root_node(&self) -> SourceTreeNode;
    fn open_file(&mut self, file_name: &str) -> Result<impl Read + Seek>;
}

pub trait DsiSource {
    fn nds_source(&mut self) -> &mut impl NdsSource;
    fn open_arm9i(&mut self) -> Result<impl Read + Seek>;
    fn open_arm7i(&mut self) -> Result<impl Read + Seek>;
}

pub enum Source<T: NdsSource, U: DsiSource> {
    Nds(T),
    Dsi(U),
}