// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use rkyv::{
    Archive, Deserialize, Serialize,
    api::high::{HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    de::Pool,
    rancor::{Error as RkyvError, Strategy},
    ser::allocator::ArenaHandle,
    util::AlignedVec,
};
use std::{fmt, mem::size_of};

const MAGIC: u64 = 0x9B98E2A6896DC3E1;
const DATA_BLOCKS_OFFS: u64 = 16;
pub const BLOCK_SIZE: usize = 512;

#[inline]
fn from_le(bytes: &[u8], nr: usize) -> u64 {
    match nr {
        1 => bytes[0] as u64,
        2 => u16::from_le_bytes(bytes[0..2].try_into().unwrap()) as u64,
        4 => u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as u64,
        8 => u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        _ => unreachable!(),
    }
}

#[inline]
fn to_le(bytes: &mut Vec<u8>, nr: usize, val: u64) {
    match nr {
        1 => bytes.extend_from_slice(&[val as u8]),
        2 => bytes.extend_from_slice(&(val as u16).to_le_bytes()),
        4 => bytes.extend_from_slice(&(val as u32).to_le_bytes()),
        8 => bytes.extend_from_slice(&val.to_le_bytes()),
        _ => unreachable!(),
    }
}

pub type Block = [u8; BLOCK_SIZE];

pub trait BlockIo {
    type Error;
    fn num_blocks(&self) -> Result<u64, Self::Error>;
    fn read_block(&mut self, index: u64) -> Result<Block, Self::Error>;
    fn write_block(&mut self, index: u64, block: Block) -> Result<(), Self::Error>;
}

pub trait Item:
    Archive + for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, RkyvError>>
{
    fn is_valid(&self) -> bool;
    fn max_size() -> usize;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    InvalidNumBlocks,
    BlockRead,
    BlockWrite,
    BlockDeviceFull,
    QueueFull,
    StatusPageFormat,
    ItemSerialize,
    ItemSerializeSize,
    ItemDeserialize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

#[derive(Archive, Serialize, Deserialize, Default)]
struct StatusPage {
    magic0: u64,
    serial0: u64,

    read_block: u64,
    write_block: u64,
    read_byte: u16,
    write_byte: u16,

    serial1: u64,
    magic1: u64,
}

const STATUS_PAGE_SIZE: usize = size_of::<rkyv::Archived<StatusPage>>();

pub struct SdLog<B: BlockIo, I: Item, const QLEN: usize> {
    bio: B,
    queue: heapless::Deque<I, QLEN>,

    /// Bytes of the block currently being assembled for writing (`write_block`).
    /// Its length always equals `write_byte` and is always `< BLOCK_SIZE`.
    wrbuf: heapless::Vec<u8, BLOCK_SIZE>,

    /// Cache of the block currently loaded for reading, so that multiple
    /// items packed into the same block don't cause repeated device reads.
    rdcache: Block,
    rdcache_idx: Option<u64>,

    status_page_dirty: bool,
    serial: u64,

    read_block: u64,
    read_byte: u16,
    write_block: u64,
    write_byte: u16,

    num_blocks: u64,
}

impl<B: BlockIo, I: Item, const QLEN: usize> SdLog<B, I, QLEN>
where
    I::Archived: for<'a> CheckBytes<HighValidator<'a, RkyvError>>
        + Deserialize<I, Strategy<Pool, RkyvError>>,
{
    pub fn new(bio: B) -> Result<Self, Error> {
        let num_blocks = bio.num_blocks().map_err(|_| Error::InvalidNumBlocks)?;
        if num_blocks < 1024 {
            return Err(Error::InvalidNumBlocks);
        }

        let mut this = Self {
            bio,
            queue: heapless::Deque::new(),
            wrbuf: heapless::Vec::new(),
            rdcache: [0; BLOCK_SIZE],
            rdcache_idx: None,
            status_page_dirty: false,
            serial: 0,
            read_block: 0,
            read_byte: 0,
            write_block: 0,
            write_byte: 0,
            num_blocks,
        };

        this.sd_read_status_page()?;

        // Reload the bytes of the in-progress write block from the device.
        if this.write_byte > 0 {
            let block = this
                .bio
                .read_block(this.write_block + DATA_BLOCKS_OFFS)
                .map_err(|_| Error::BlockRead)?;
            let result = this
                .wrbuf
                .extend_from_slice(&block[..this.write_byte as usize]);
            assert!(result.is_ok(), "wrbuf overflow when loading write block");
        }

        Ok(this)
    }

    fn item_prefix_size() -> usize {
        let max_size = I::max_size();
        if usize::MAX >= u8::MAX as usize && max_size <= u8::MAX as usize {
            1
        } else if usize::MAX >= u16::MAX as usize && max_size <= u16::MAX as usize {
            2
        } else if usize::MAX >= u32::MAX as usize && max_size <= u32::MAX as usize {
            4
        } else if usize::MAX >= u64::MAX as usize && max_size <= u64::MAX as usize {
            8
        } else {
            panic!("Item max size is too big.");
        }
    }

    fn block_idx_inc(&self, index: u64) -> u64 {
        (index + 1) % (self.num_blocks - DATA_BLOCKS_OFFS)
    }

    fn block_idx_is_valid(&self, index: u64) -> bool {
        index < self.num_blocks - DATA_BLOCKS_OFFS
    }

    /// Total number of usable data bytes on the device.
    fn capacity_bytes(&self) -> u64 {
        (self.num_blocks - DATA_BLOCKS_OFFS) * BLOCK_SIZE as u64
    }

    /// Absolute byte offset, within the (wrapped) data area, of `(block,
    /// byte)`.
    fn linear_pos(&self, block: u64, byte: u16) -> u64 {
        block * BLOCK_SIZE as u64 + byte as u64
    }

    /// Number of unread bytes currently stored between the read and write positions.
    fn occupancy(&self) -> u64 {
        let cap = self.capacity_bytes();
        let w = self.linear_pos(self.write_block, self.write_byte);
        let r = self.linear_pos(self.read_block, self.read_byte);
        (w + cap - r) % cap
    }

    /// Number of bytes that can still be written without catching up with
    /// the read position. One byte of capacity is permanently reserved so
    /// that the "empty" and "full" states of the ring buffer can always be
    /// told apart.
    fn free_bytes(&self) -> u64 {
        self.capacity_bytes() - 1 - self.occupancy()
    }

    fn sd_read_status_page_at(&mut self, block_index: u64) -> Result<StatusPage, Error> {
        if let Ok(block) = self.bio.read_block(block_index) {
            let stat = rkyv::from_bytes::<StatusPage, RkyvError>(&block[..STATUS_PAGE_SIZE]);
            if let Ok(stat) = stat {
                if stat.magic0 == MAGIC
                    && stat.magic0 == stat.magic1
                    && stat.serial0 == stat.serial1
                    && self.block_idx_is_valid(stat.read_block)
                    && self.block_idx_is_valid(stat.write_block)
                    && (stat.read_byte as usize) < BLOCK_SIZE
                    && (stat.write_byte as usize) < BLOCK_SIZE
                {
                    Ok(stat)
                } else {
                    Ok(Default::default())
                }
            } else {
                Ok(Default::default())
            }
        } else {
            Err(Error::BlockRead)
        }
    }

    fn sd_read_status_page(&mut self) -> Result<(), Error> {
        let stat0 = self.sd_read_status_page_at(0)?;
        let stat1 = self.sd_read_status_page_at(1)?;

        let stat = if stat0.serial0 >= stat1.serial0 {
            #[cfg(feature = "debug")]
            println!("SdLog: Using status page 0.");
            &stat0
        } else {
            #[cfg(feature = "debug")]
            println!("SdLog: Using status page 1.");
            &stat1
        };

        self.serial = stat.serial0;
        self.read_block = stat.read_block;
        self.read_byte = stat.read_byte;
        self.write_block = stat.write_block;
        self.write_byte = stat.write_byte;
        self.status_page_dirty = false;

        Ok(())
    }

    fn sd_write_status_page(&mut self) -> Result<(), Error> {
        assert!(self.serial < u64::MAX);
        self.serial += 1;

        let stat = StatusPage {
            magic0: MAGIC,
            magic1: MAGIC,
            serial0: self.serial,
            serial1: self.serial,
            read_block: self.read_block,
            write_block: self.write_block,
            read_byte: self.read_byte,
            write_byte: self.write_byte,
        };

        let bytes = rkyv::to_bytes::<RkyvError>(&stat).map_err(|_| Error::StatusPageFormat)?;
        if bytes.len() > BLOCK_SIZE {
            return Err(Error::StatusPageFormat);
        }

        let mut block: Block = [0; BLOCK_SIZE];
        block[..bytes.len()].copy_from_slice(&bytes);

        let block_index = self.serial & 1; // Block 0 or 1.
        self.bio
            .write_block(block_index, block)
            .map_err(|_| Error::BlockWrite)?;

        Ok(())
    }

    /// Flush the currently full `wrbuf` to the device as a complete block
    /// and advance the write position to the next block.
    fn flush_full_block(&mut self) -> Result<(), Error> {
        #[cfg(feature = "debug")]
        assert_eq!(self.wrbuf.len(), BLOCK_SIZE);

        let mut block: Block = [0; BLOCK_SIZE];
        block.copy_from_slice(&self.wrbuf);
        self.bio
            .write_block(self.write_block + DATA_BLOCKS_OFFS, block)
            .map_err(|_| Error::BlockWrite)?;

        #[cfg(feature = "debug")]
        println!("SdLog: Wrote full block {} to device.", self.write_block);

        self.write_block = self.block_idx_inc(self.write_block);
        self.write_byte = 0;
        self.wrbuf.clear();

        Ok(())
    }

    /// Append `buf` to the write stream, transparently flushing complete
    /// blocks to the device as they fill up.
    fn write_data(&mut self, mut buf: &[u8]) -> Result<(), Error> {
        while !buf.is_empty() {
            let space = BLOCK_SIZE - self.wrbuf.len();
            let n = space.min(buf.len());
            let result = self.wrbuf.extend_from_slice(&buf[..n]);
            assert!(result.is_ok(), "wrbuf overflow when writing data");
            buf = &buf[n..];
            self.write_byte = self.wrbuf.len() as u16;
            if self.wrbuf.len() == BLOCK_SIZE {
                self.flush_full_block()?;
            }
            self.status_page_dirty = true;
        }
        Ok(())
    }

    pub fn push_item(&mut self, item: I) -> Result<(), Error> {
        self.queue.push_back(item).map_err(|_| Error::QueueFull)
    }

    /// Serialize `item` and frame it with a little-endian length prefix.
    fn serialize_item(item: &I) -> Result<Vec<u8>, Error> {
        let bytes = rkyv::to_bytes::<RkyvError>(item).map_err(|_| Error::ItemSerialize)?;

        let len = bytes.len();
        if len == 0 || len > u16::MAX as usize {
            return Err(Error::ItemSerializeSize);
        }

        let prefix_size = Self::item_prefix_size();
        let mut framed = Vec::with_capacity(prefix_size + len);
        to_le(&mut framed, prefix_size, len as u64);
        framed.extend_from_slice(&bytes);
        Ok(framed)
    }

    /// Serialize and pack `item` into the write stream.
    fn write_item(&mut self, item: &I) -> Result<(), Error> {
        let framed = Self::serialize_item(item)?;
        let needed = framed.len() as u64;

        if needed > I::max_size() as u64 {
            return Err(Error::ItemSerializeSize);
        }
        if needed > self.capacity_bytes().saturating_sub(1) {
            return Err(Error::ItemSerializeSize);
        }
        if needed > self.free_bytes() {
            return Err(Error::BlockDeviceFull);
        }

        self.write_data(&framed)
    }

    fn flush_queue(&mut self) -> Result<(), Error> {
        while let Some(item) = self.queue.pop_front() {
            if let Err(e) = self.write_item(&item) {
                // Put the item back.
                let _ = self.queue.push_front(item);
                return Err(e);
            }
        }
        Ok(())
    }

    /// Ensure the block at `self.read_block` is loaded into `self.rdcache`.
    fn load_read_cache(&mut self) -> Result<(), Error> {
        if self.rdcache_idx != Some(self.read_block) {
            self.rdcache = self
                .bio
                .read_block(self.read_block + DATA_BLOCKS_OFFS)
                .map_err(|_| Error::BlockRead)?;
            self.rdcache_idx = Some(self.read_block);
        }
        Ok(())
    }

    /// Read exactly `n` bytes from the read stream, starting at the
    /// current read position, transparently crossing block boundaries.
    fn read_stream(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let mut out = Vec::with_capacity(n);

        while out.len() < n {
            if self.read_block == self.write_block {
                // The block being read is still the one being assembled
                // for writing: take the bytes straight from `wrbuf`
                // instead of the (possibly stale) on-device block.
                let start = self.read_byte as usize;
                let avail = self.wrbuf.len().saturating_sub(start);
                if avail == 0 {
                    return Err(Error::ItemDeserialize);
                }
                let take = avail.min(n - out.len());
                out.extend_from_slice(&self.wrbuf[start..start + take]);
                self.read_byte += take as u16;
            } else {
                self.load_read_cache()?;
                let start = self.read_byte as usize;
                let take = (BLOCK_SIZE - start).min(n - out.len());
                out.extend_from_slice(&self.rdcache[start..start + take]);
                self.read_byte += take as u16;

                if self.read_byte as usize == BLOCK_SIZE {
                    self.read_block = self.block_idx_inc(self.read_block);
                    self.read_byte = 0;
                }
            }
        }

        Ok(out)
    }

    /// Like [`Self::read_stream`], but does not advance the read position.
    fn peek_stream(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let saved_block = self.read_block;
        let saved_byte = self.read_byte;
        let result = self.read_stream(n);
        self.read_block = saved_block;
        self.read_byte = saved_byte;
        result
    }

    pub fn pop_item(&mut self) -> Result<Option<I>, Error> {
        let prefix_size = Self::item_prefix_size();
        let avail = self.occupancy();
        if avail < prefix_size as u64 {
            return Ok(None);
        }

        let len_bytes = self.peek_stream(prefix_size)?;
        let len = from_le(&len_bytes, prefix_size) as usize;
        let total = prefix_size + len;

        if len == 0 || avail < total as u64 {
            // Not a complete item yet.
            return Ok(None);
        }

        let bytes = self.read_stream(total)?;
        self.status_page_dirty = true;

        #[cfg(feature = "debug")]
        println!("SdLog: Read item ({len} bytes) from device.");

        match rkyv::from_bytes::<I, RkyvError>(&bytes[prefix_size..]) {
            Ok(item) => {
                if item.is_valid() {
                    Ok(Some(item))
                } else {
                    Err(Error::ItemDeserialize)
                }
            }
            Err(_) => Err(Error::ItemDeserialize),
        }
    }

    /// Persist all data written so far,
    /// including the not-yet-full tail block, and the status page.
    pub fn commit(&mut self) -> Result<(), Error> {
        self.flush_queue()?;

        if !self.wrbuf.is_empty() {
            let mut block: Block = [0; BLOCK_SIZE];
            block[..self.wrbuf.len()].copy_from_slice(&self.wrbuf);
            self.bio
                .write_block(self.write_block + DATA_BLOCKS_OFFS, block)
                .map_err(|_| Error::BlockWrite)?;
            #[cfg(feature = "debug")]
            println!(
                "SdLog: Persisted partial block {} ({} bytes) to device.",
                self.write_block,
                self.wrbuf.len()
            );
        }

        if self.status_page_dirty {
            self.sd_write_status_page()?;
            self.status_page_dirty = false;
        }
        Ok(())
    }

    pub fn get_read_block_index(&self) -> u64 {
        self.read_block
    }

    pub fn get_write_block_index(&self) -> u64 {
        self.write_block
    }

    pub fn get_read_byte_index(&self) -> u16 {
        self.read_byte
    }

    pub fn get_write_byte_index(&self) -> u16 {
        self.write_byte
    }

    pub fn get_num_blocks(&self) -> u64 {
        self.num_blocks
    }
}

// vim: ts=4 sw=4 expandtab
