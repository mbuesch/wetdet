// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use anyhow as ah;
use clap::Parser;
use libc::{S_IFBLK, S_IFMT};
use logentry::LogEntry;
use sdlog::{BLOCK_SIZE, Block, BlockIo, SdLog};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::{fs::MetadataExt, io::AsRawFd},
    path::{Path, PathBuf},
};

nix::ioctl_read!(blkgetsize64, 0x12, 114, u64);

struct Io {
    file: File,
}

impl Io {
    fn new(path: &Path, write: bool) -> ah::Result<Self> {
        let file = OpenOptions::new().read(true).write(write).open(path)?;
        Ok(Self { file })
    }
}

impl BlockIo for Io {
    type Error = ah::Error;

    fn num_blocks(&self) -> Result<u64, Self::Error> {
        let meta = self.file.metadata()?;
        if meta.mode() & S_IFMT == S_IFBLK {
            let fd = self.file.as_raw_fd();
            let mut len = 0_u64;
            unsafe { blkgetsize64(fd, &mut len as *mut u64) }?;
            Ok(len / BLOCK_SIZE as u64)
        } else {
            Ok(meta.len() / BLOCK_SIZE as u64)
        }
    }

    fn read_block(&mut self, index: u64) -> Result<Block, Self::Error> {
        let byte_offs = index * BLOCK_SIZE as u64;
        self.file.seek(SeekFrom::Start(byte_offs))?;
        let mut block = [0_u8; BLOCK_SIZE];
        self.file.read_exact(&mut block)?;
        Ok(block)
    }

    fn write_block(&mut self, index: u64, block: Block) -> Result<(), Self::Error> {
        let byte_offs = index * BLOCK_SIZE as u64;
        self.file.seek(SeekFrom::Start(byte_offs))?;
        self.file.write_all(&block)?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
struct Args {
    /// The device file of the SD card.
    dev: PathBuf,

    /// Commit the read entries to the SD card.
    /// All operations are read-only *unless* the '--commit' option is used.
    /// '--commit' will update the read block index on the SD card.
    #[arg(long)]
    commit: bool,

    /// Do not abort on entry errors.
    #[arg(short = 'c', long)]
    error_continue: bool,
}

fn handle_err(args: &Args, msg: &str) -> ah::Result<()> {
    if args.error_continue {
        eprintln!("{}", msg);
        Ok(())
    } else {
        Err(ah::format_err!("{}", msg))
    }
}

fn main() -> ah::Result<()> {
    let args = Args::parse();

    let io = Io::new(&args.dev, args.commit)?;
    let mut sdlog = SdLog::<Io, LogEntry, 8>::new(io)?;

    eprintln!(
        "MeasLog: read_block_index = {}, write_block_index = {}, num_blocks = {}",
        sdlog.get_read_block_index(),
        sdlog.get_write_block_index(),
        sdlog.get_num_blocks()
    );

    loop {
        match sdlog.pop_item() {
            Ok(None) => break,
            Ok(Some(s)) => {
                if let Some(csv) = s.format_csv() {
                    print!("{csv}");
                } else {
                    handle_err(&args, "MeasLog: Entry is invalid.")?;
                }
            }
            Err(sdlog::Error::ItemDeserialize) => (),
            Err(e) => {
                handle_err(&args, &format!("MeasLog: Failed to read entry: {e:?}"))?;
            }
        }
    }

    if args.commit
        && let Err(e) = sdlog.commit()
    {
        handle_err(&args, &format!("MeasLog: Failed to commit: {e:?}"))?;
    }

    Ok(())
}

// vim: ts=4 sw=4 expandtab
