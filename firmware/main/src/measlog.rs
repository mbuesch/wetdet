// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use embedded_sdmmc::{Block, BlockDevice, BlockIdx, SdCard};
use esp_idf_hal::{
    delay,
    gpio::AnyOutputPin,
    spi::{self, SpiDeviceDriver, SpiDriver},
    units::KiloHertz,
};
use rkyv::{Archive, Deserialize, Serialize};
use std::sync::Arc;

const BAUD: KiloHertz = KiloHertz(10000);
const QUEUE_LEN: usize = 61;

#[derive(Debug, Clone, Copy, Archive, Deserialize, Serialize)]
pub struct LogEntry {
    valid: u32,
    pub boot_id: u32,
    pub serial: u32,
    pub temp_c: u32,
    pub pres_hpa: u32,
    pub rel_hum: u32,
}

impl LogEntry {
    const VALID: u32 = 0xA5A5A5A5;
    const FACT: f32 = 1000.0;

    pub const fn new(boot_id: u32, serial: u32, temp_c: f32, pres_hpa: f32, rel_hum: f32) -> Self {
        LogEntry {
            valid: Self::VALID,
            boot_id,
            serial,
            temp_c: (temp_c * Self::FACT) as u32,
            pres_hpa: (pres_hpa * Self::FACT) as u32,
            rel_hum: (rel_hum * Self::FACT) as u32,
        }
    }
}

impl sdlog::Item for LogEntry {
    fn is_valid(&self) -> bool {
        self.valid == Self::VALID
    }

    fn max_size() -> usize {
        u8::MAX as usize
    }
}

pub struct MeasLogDevice<'a> {
    sd: SdCard<SpiDeviceDriver<'a, Arc<SpiDriver<'a>>>, delay::Ets>,
    sd_size: u64,
}

impl<'a> MeasLogDevice<'a> {
    pub fn new(spi_drv: Arc<SpiDriver<'a>>, cs: AnyOutputPin<'a>) -> Self {
        let spi_sd_config = spi::config::Config::new()
            .baudrate(BAUD.into())
            .data_mode(spi::config::Mode {
                polarity: spi::config::Polarity::IdleHigh,
                phase: spi::config::Phase::CaptureOnSecondTransition,
            })
            .write_only(false)
            .duplex(spi::config::Duplex::Full)
            .bit_order(spi::config::BitOrder::MsbFirst);
        let spi_sd_drv = SpiDeviceDriver::new(spi_drv, Some(cs), &spi_sd_config).unwrap();
        let delayer = delay::Ets;
        let sd = SdCard::new(spi_sd_drv, delayer);
        let sd_size = sd
            .num_bytes()
            .expect("MeasLog: Failed to initialize SD card");
        let max_sd_size = u32::MAX as u64 * Block::LEN as u64;
        let sd_size = if sd_size > max_sd_size {
            println!(
                "MeasLog: SD card too large ({sd_size} bytes). \
                Limiting to {max_sd_size} bytes.",
            );
            max_sd_size
        } else {
            println!("MeasLog: SD size {sd_size} bytes.");
            sd_size
        };
        Self { sd, sd_size }
    }
}

impl<'a> sdlog::BlockIo for MeasLogDevice<'a> {
    type Error = ();

    fn num_blocks(&self) -> Result<u64, Self::Error> {
        let num_blocks = self.sd_size / sdlog::BLOCK_SIZE as u64;
        if num_blocks > u32::MAX as u64 {
            Err(())
        } else {
            Ok(num_blocks)
        }
    }

    fn read_block(&mut self, index: u64) -> Result<sdlog::Block, Self::Error> {
        if index > u32::MAX as u64 {
            Err(())
        } else {
            let mut block: [Block; 1] = [Default::default(); 1];
            self.sd
                .read(&mut block, BlockIdx(index as u32))
                .map_err(|_| ())?;
            Ok(block[0].contents)
        }
    }

    fn write_block(&mut self, index: u64, block: sdlog::Block) -> Result<(), Self::Error> {
        if index > u32::MAX as u64 {
            Err(())
        } else {
            let block = [Block { contents: block }; 1];
            self.sd
                .write(&block, BlockIdx(index as u32))
                .map_err(|_| ())
        }
    }
}

pub struct MeasLog<'a> {
    log: sdlog::SdLog<MeasLogDevice<'a>, LogEntry, QUEUE_LEN>,
}

impl<'a> MeasLog<'a> {
    pub fn new(spi_drv: Arc<SpiDriver<'a>>, cs: AnyOutputPin<'a>) -> Self {
        let log = match sdlog::SdLog::new(MeasLogDevice::new(spi_drv, cs)) {
            Ok(l) => l,
            Err(e) => panic!("MeasLog: Init failed: {e:?}"),
        };
        println!(
            "MeasLog: read_block_idx={}, write_block_idx={}, num_blocks={}",
            log.get_read_block_index(),
            log.get_write_block_index(),
            log.get_num_blocks()
        );
        Self { log }
    }

    pub fn push_entry(&mut self, entry: LogEntry) {
        if let Err(e) = self.log.push_item(entry) {
            println!("MeasLog: Failed to queue entry: {e:?}");
        }
    }

    pub fn commit(&mut self) {
        if let Err(e) = self.log.flush_queue() {
            println!("MeasLog: Queue processing failed: {e:?}");
        }
        if let Err(e) = self.log.commit() {
            println!("MeasLog: Commit failed: {e:?}");
        }
    }
}

// vim: ts=4 sw=4 expandtab
