use std::{
    fs::File,
    io::{Read, Write, copy},
    path::{Path, PathBuf},
};

use crate::{Result, blz_decompress, util::mkdir};
use crate::{
    overlay::format_overlay_name,
    source::{DsiSource, NdsSource, Source, SourceTreeNode},
};

pub fn write_to_dir<T: NdsSource, U: DsiSource>(
    source: &mut Source<T, U>,
    out_dir: impl AsRef<Path>,
    decompress: bool,
) -> Result<()> {
    match source {
        Source::Nds(nds_source) => write_nds_to_dir(nds_source, out_dir, decompress),
        Source::Dsi(dsi_source) => write_dsi_to_dir(dsi_source, out_dir, decompress),
    }
}

fn write_nds_to_dir(
    source: &mut impl NdsSource,
    out_dir: impl AsRef<Path>,
    decompress: bool,
) -> Result<()> {
    let out_dir = out_dir.as_ref();
    mkdir(out_dir)?;

    let out_path: PathBuf = [out_dir, Path::new("arm9.bin")].iter().collect();
    if decompress {
        decompress_and_write(&mut source.open_arm9()?, &out_path)?;
    } else {
        copy(&mut source.open_arm9()?, &mut File::create(out_path)?)?;
    }

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
    copy(
        &mut source.open_arm9_overlay_table()?,
        &mut File::create(out_path)?,
    )?;

    let out_path: PathBuf = [out_dir, Path::new("arm7_overlay_table.bin")]
        .iter()
        .collect();
    copy(
        &mut source.open_arm7_overlay_table()?,
        &mut File::create(out_path)?,
    )?;

    let out_path: PathBuf = [out_dir, Path::new("overlay")].iter().collect();
    write_overlays(source, &out_path, decompress)?;

    let out_path: PathBuf = [out_dir, Path::new("data")].iter().collect();
    mkdir(&out_path)?;
    write_node(source, &out_path, None, &source.root_node())?;

    Ok(())
}

fn write_dsi_to_dir(
    source: &mut impl DsiSource,
    out_dir: impl AsRef<Path>,
    decompress: bool,
) -> Result<()> {
    let out_dir = out_dir.as_ref();
    write_nds_to_dir(source.nds_source(), out_dir, decompress)?;

    let out_path: PathBuf = [out_dir, Path::new("arm9i.bin")].iter().collect();
    copy(&mut source.open_arm9i()?, &mut File::create(out_path)?)?;

    let out_path: PathBuf = [out_dir, Path::new("arm7i.bin")].iter().collect();
    copy(&mut source.open_arm7i()?, &mut File::create(out_path)?)?;

    Ok(())
}

fn write_overlays(source: &mut impl NdsSource, out_dir: &Path, decompress: bool) -> Result<()> {
    mkdir(out_dir)?;
    let arm9_overlay_count = source.arm9_overlay_count();
    for overlay_id in 0..arm9_overlay_count {
        let overlay_entry = source.arm9_overlay_metadata(overlay_id);
        let out_path: PathBuf = [out_dir, &format_overlay_name(overlay_id)].iter().collect();
        if decompress && overlay_entry.is_compressed() {
            decompress_and_write(&mut source.open_arm9_overlay(overlay_id)?, &out_path)?;
        } else {
            copy(
                &mut source.open_arm9_overlay(overlay_id)?,
                &mut File::create(out_path)?,
            )?;
        }
    }
    let arm7_overlay_count = source.arm7_overlay_count();
    for overlay_id in 0..arm7_overlay_count {
        let out_path: PathBuf = [
            out_dir,
            &format_overlay_name(arm9_overlay_count + overlay_id),
        ]
        .iter()
        .collect();
        copy(
            &mut source.open_arm7_overlay(overlay_id)?,
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

fn decompress_and_write(reader: &mut impl Read, out_path: &Path) -> Result<()> {
    let mut compressed_buffer = Vec::new();
    reader.read_to_end(&mut compressed_buffer)?;
    let decompressed_buffer = blz_decompress(&compressed_buffer)?;
    let mut writer = File::create(out_path)?;
    writer.write_all(&decompressed_buffer)?;
    Ok(())
}
