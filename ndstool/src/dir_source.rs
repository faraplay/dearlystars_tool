use std::{
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use crate::Result;
use crate::{
    error::NdsError,
    overlay::format_overlay_string,
    source::{DsiSource, NdsSource, Source, SourceTreeNode},
};

pub struct NdsDsiDirSource {
    base_path: PathBuf,
    arm9_overlay_count: u16,
    arm7_overlay_count: u16,
    root_node: SourceTreeNode,
}

type DirSource = Source<NdsDsiDirSource, NdsDsiDirSource>;

pub fn read_from_dir(path: impl AsRef<Path>) -> Result<DirSource> {
    let mut source = NdsDsiDirSource::read_path(path.as_ref())?;
    let mut header_reader = source.open_header()?;
    let size = header_reader.seek(std::io::SeekFrom::End(0))?;
    drop(header_reader);
    if size > 0x200 {
        Ok(Source::Dsi(source))
    } else {
        Ok(Source::Nds(source))
    }
}

impl NdsDsiDirSource {
    fn open_path(&self, path: &str) -> Result<File> {
        let combined_path: PathBuf = [&self.base_path, Path::new(path)].iter().collect();
        Ok(File::open(&combined_path)?)
    }

    fn read_path(path: &Path) -> Result<NdsDsiDirSource> {
        let base_path = PathBuf::from(path);
        let arm9_overlay_count = get_overlay_count(&base_path, "arm9_overlay_table.bin")?;
        let arm7_overlay_count = get_overlay_count(&base_path, "arm7_overlay_table.bin")?;
        let data_path: PathBuf = [&base_path, Path::new("data")].iter().collect();
        let root_node = read_path_into_node(&data_path)?;
        Ok(NdsDsiDirSource {
            base_path,
            arm9_overlay_count,
            arm7_overlay_count,
            root_node,
        })
    }
}

impl NdsSource for NdsDsiDirSource {
    fn open_arm9(&mut self) -> Result<impl Read + Seek> {
        self.open_path("arm9.bin")
    }
    fn open_arm7(&mut self) -> Result<impl Read + Seek> {
        self.open_path("arm7.bin")
    }
    fn open_banner(&mut self) -> Result<impl Read + Seek> {
        self.open_path("banner.bin")
    }
    fn open_header(&mut self) -> Result<impl Read + Seek> {
        self.open_path("header.bin")
    }
    fn open_logo(&mut self) -> Result<impl Read + Seek> {
        self.open_path("logo.bin")
    }
    fn open_arm9_overlay_table(&mut self) -> Result<impl Read + Seek> {
        self.open_path("arm9_overlay_table.bin")
    }
    fn open_arm7_overlay_table(&mut self) -> Result<impl Read + Seek> {
        self.open_path("arm7_overlay_table.bin")
    }
    fn arm9_overlay_count(&self) -> u16 {
        self.arm9_overlay_count
    }
    fn open_arm9_overlay(&mut self, overlay_index: u16) -> Result<impl Read + Seek> {
        self.open_path(&format!("overlay/{}", format_overlay_string(overlay_index)))
    }
    fn arm7_overlay_count(&self) -> u16 {
        self.arm7_overlay_count
    }
    fn open_arm7_overlay(&mut self, overlay_index: u16) -> Result<impl Read + Seek> {
        self.open_path(&format!(
            "overlay/{}",
            format_overlay_string(self.arm9_overlay_count + overlay_index)
        ))
    }
    fn root_node(&self) -> SourceTreeNode {
        self.root_node.clone()
    }
    fn open_file(&mut self, file_name: &str) -> Result<impl Read + Seek> {
        self.open_path(&format!("data/{file_name}"))
    }
}

impl DsiSource for NdsDsiDirSource {
    fn nds_source(&mut self) -> &mut impl NdsSource {
        self
    }
    fn open_arm9i(&mut self) -> Result<impl Read + Seek> {
        self.open_path("arm9i.bin")
    }
    fn open_arm7i(&mut self) -> Result<impl Read + Seek> {
        self.open_path("arm7i.bin")
    }
}

fn get_overlay_count(base_path: &Path, overlay_table_file_name: &str) -> Result<u16> {
    let overlay_path: PathBuf = [base_path, Path::new(overlay_table_file_name)]
        .iter()
        .collect();
    let mut reader = File::open(overlay_path)?;
    let size = reader.seek(std::io::SeekFrom::End(0))?;
    Ok((size / 0x20) as u16)
}

fn file_name(path: &Path) -> Result<String> {
    let os_str = match path.file_name() {
        Some(os_str) => os_str,
        None => {
            return Err(NdsError {
                message: format!("Directory path {} ends in ..", path.display()),
            }
            .into());
        }
    };
    let name: String = match os_str.to_str() {
        Some(s) => s,
        None => {
            return Err(NdsError {
                message: format!("Directory name {} is not valid ASCII", &os_str.display()),
            }
            .into());
        }
    }
    .into();
    if !name.is_ascii() {
        return Err(NdsError {
            message: format!("Directory name {} is not valid ASCII", &os_str.display()),
        }
        .into());
    }
    Ok(name)
}

fn read_path_into_node(path: &Path) -> Result<SourceTreeNode> {
    let name = file_name(path)?;
    if path.is_file() {
        return Ok(SourceTreeNode {
            name,
            children: None,
        });
    } else if path.is_dir() {
        let mut child_nodes = path
            .read_dir()?
            .map(|dir_entry| read_path_into_node(&dir_entry?.path()))
            .collect::<Result<Vec<_>>>()?;
        child_nodes.sort_unstable_by(|a, b| {
            a.name
                .to_ascii_uppercase()
                .cmp(&b.name.to_ascii_uppercase())
        });
        return Ok(SourceTreeNode {
            name,
            children: Some(child_nodes),
        });
    } else {
        return Err(NdsError {
            message: format!("Item {} is not a file or directory", path.display()),
        }
        .into());
    }
}
