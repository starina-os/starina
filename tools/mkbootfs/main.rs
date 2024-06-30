use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::mem::size_of;
use std::path::PathBuf;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use bytes::BufMut;
use bytes::BytesMut;
use clap::Parser;
use ftl_types::bootfs::BootfsEntry;
use ftl_types::bootfs::BootfsHeader;
use ftl_types::bootfs::EntryType;
use ftl_types::bootfs::BOOTFS_MAGIC;
use ftl_types::bootfs::NAME_LEN_MAX;
use ftl_utils::alignment::align_up;
use ftl_utils::alignment::is_aligned;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short)]
    outfile: PathBuf,
    #[arg(index = 1)]
    indir: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.indir.is_dir() {
        bail!("{} is not a directory", args.indir.display());
    }

    let mut entries = Vec::new();
    for entry in WalkDir::new(&args.indir) {
        let path = entry?.into_path();
        if !path.is_file() {
            continue;
        }

        // TODO: Don't read the whole file into memory. I bet there's a
        //       better way to build the image :/
        let mut filedata: Vec<u8> = Vec::new();
        OpenOptions::new()
            .read(true)
            .open(&path)
            .context("failed to open file")?
            .read_to_end(&mut filedata)
            .context("failed to read file")?;

        let padding = align_up(filedata.len(), 4096) - filedata.len();
        entries.push((path, filedata, padding));
    }

    let mut image = BytesMut::new();

    // BootFS header.
    image.put_slice(BOOTFS_MAGIC.as_slice());
    image.put_u32_le(entries.len().try_into().unwrap());

    // Entries.
    let mut offset = align_up(
        size_of::<BootfsHeader>() + entries.len() * size_of::<BootfsEntry>(),
        4096,
    );
    for (path, filedata, padding) in &entries {
        let name =  path
            .strip_prefix(&args.indir)
            .context("failed to strip prefix")?
            .to_str()
            .context("failed to convert to str")?
            .to_owned();

        // Subtract 1 for the null terminator.
        if name.as_bytes().len() > NAME_LEN_MAX - 1 {
            bail!("too long path: {}", path.display());
        }

        // size
        image.put_u32_le(filedata.len().try_into().unwrap());
        // offset
        image.put_u32_le(offset.try_into().unwrap());
        // entry_type
        image.put_u8(EntryType::File as u8);
        // name
        image.put_slice(name.as_bytes());
        image.put_bytes(0, NAME_LEN_MAX - name.as_bytes().len());

        offset += filedata.len() + padding;
        debug_assert!(is_aligned(offset, 4096));
    }

    image.put_bytes(0, align_up(image.len(), 4096) - image.len());

    // File data.
    for (_, filedata, padding) in &entries {
        image.put_slice(filedata);
        image.put_bytes(0, *padding);
    }

    let mut tmpfile = tempfile::NamedTempFile::new().context("failed to create temporary file")?;
    tmpfile.write_all(&image)?;
    tmpfile
        .persist(args.outfile)
        .context("failed to persist file")?;
    Ok(())
}
