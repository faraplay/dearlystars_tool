use std::{
    fs::File,
    io::copy,
    path::{Path, PathBuf},
};

use crate::{
    Result,
    overlay::{OverlayEntry, format_overlay_name, write_overlay_table},
    source::{DsiSource, NdsSource, Source, SourceTreeNode},
    util::mkdir,
};

pub fn write_to_dir<T: NdsSource, U: DsiSource>(
    source: &mut Source<T, U>,
    out_dir: impl AsRef<Path>,
) -> Result<()> {
    match source {
        Source::Nds(nds_source) => write_nds_to_dir(nds_source, out_dir),
        Source::Dsi(dsi_source) => write_dsi_to_dir(dsi_source, out_dir),
    }
}

fn write_nds_to_dir(source: &mut impl NdsSource, out_dir: impl AsRef<Path>) -> Result<()> {
    let out_dir = out_dir.as_ref();
    mkdir(out_dir)?;

    let out_path: PathBuf = [out_dir, Path::new("arm9.bin")].iter().collect();
    copy(&mut source.open_arm9()?, &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("arm7.bin")].iter().collect();
    copy(&mut source.open_arm7()?, &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("banner.bin")].iter().collect();
    copy(&mut source.open_banner()?, &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("header.bin")].iter().collect();
    copy(&mut source.open_header()?, &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("logo.bin")].iter().collect();
    copy(&mut source.open_logo()?, &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("arm9_overlay_table.bin")]
        .iter()
        .collect();
    write_overlay_table(source.arm9_overlay_metadata(), &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("arm7_overlay_table.bin")]
        .iter()
        .collect();
    write_overlay_table(source.arm7_overlay_metadata(), &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("overlay")].iter().collect();
    write_overlays(source, &out_path)?;

    let out_path: PathBuf = [out_dir, Path::new("data")].iter().collect();
    mkdir(&out_path)?;
    write_node(source, &out_path, None, &source.root_node())?;

    Ok(())
}

fn write_dsi_to_dir(source: &mut impl DsiSource, out_dir: impl AsRef<Path>) -> Result<()> {
    let out_dir = out_dir.as_ref();
    write_nds_to_dir(source.nds_source(), out_dir)?;

    let out_path: PathBuf = [out_dir, Path::new("arm9i.bin")].iter().collect();
    copy(&mut source.open_arm9i()?, &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("arm7i.bin")].iter().collect();
    copy(&mut source.open_arm7i()?, &mut File::create(out_path)?)?;

    Ok(())
}

fn write_overlays(source: &mut impl NdsSource, out_dir: &Path) -> Result<()> {
    mkdir(out_dir)?;
    let overlay_entries: Vec<OverlayEntry> = source
        .arm9_overlay_metadata()
        .chain(source.arm7_overlay_metadata())
        .cloned()
        .collect();
    for overlay_entry in overlay_entries {
        let out_path = out_dir.join(&format_overlay_name(&overlay_entry));
        copy(
            &mut source.open_overlay(&overlay_entry)?,
            &mut File::create(out_path)?,
        )?;
    }
    Ok(())
}

fn write_node(
    source: &mut impl NdsSource,
    out_dir: &Path,
    node_name: Option<&str>,
    node: &SourceTreeNode,
) -> Result<()> {
    let filesystem_path: PathBuf = [out_dir, Path::new(node_name.unwrap_or(""))]
        .iter()
        .collect();
    match &node.children {
        Some(child_nodes) => {
            mkdir(&filesystem_path)?;
            for child_node in child_nodes {
                let child_node_name: Option<&str> = Some(match node_name {
                    Some(node_name) => &format!("{}/{}", node_name, child_node.name),
                    None => &child_node.name,
                });
                write_node(source, out_dir, child_node_name, child_node)?;
            }
        }
        None => {
            let node_name = node_name.unwrap_or_default();
            copy(
                &mut source.open_file(node_name)?,
                &mut File::create(&filesystem_path)?,
            )?;
        }
    }
    Ok(())
}
