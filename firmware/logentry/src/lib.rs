// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use rkyv::{Archive, Deserialize, Serialize};
use sdlog::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Archive, Deserialize, Serialize)]
#[repr(u8)]
pub enum AlarmState {
    Off,
    OnThres,
    OnStThres,
    OnLtThres,
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
pub struct LogEntry {
    valid: u32,
    pub boot_id: u32,
    pub serial: u32,
    pub temp_c: u32,
    pub pres_hpa: u32,
    pub rel_hum: u32,
    pub alarm: AlarmState,
}

impl LogEntry {
    const VALID: u32 = 0xA5A5A5A5;
    const FACT: f32 = 1000.0;

    pub const fn new(
        boot_id: u32,
        serial: u32,
        temp_c: f32,
        pres_hpa: f32,
        rel_hum: f32,
        alarm: AlarmState,
    ) -> Self {
        LogEntry {
            valid: Self::VALID,
            boot_id,
            serial,
            temp_c: (temp_c * Self::FACT) as u32,
            pres_hpa: (pres_hpa * Self::FACT) as u32,
            rel_hum: (rel_hum * Self::FACT) as u32,
            alarm,
        }
    }

    pub fn boot_id(&self) -> u32 {
        self.boot_id
    }

    pub fn serial(&self) -> u32 {
        self.serial
    }

    pub fn temp_c(&self) -> f32 {
        self.temp_c as f32 / Self::FACT
    }

    pub fn pres_hpa(&self) -> f32 {
        self.pres_hpa as f32 / Self::FACT
    }

    pub fn rel_hum(&self) -> f32 {
        self.rel_hum as f32 / Self::FACT
    }

    pub fn alarm(&self) -> AlarmState {
        self.alarm
    }

    pub fn format_csv(&self) -> Option<String> {
        if self.is_valid() {
            Some(format!(
                "{:08X},{:08X},{:.1},{:.1},{:.1},{}\n",
                self.boot_id(),
                self.serial(),
                self.temp_c(),
                self.pres_hpa(),
                self.rel_hum(),
                match self.alarm() {
                    AlarmState::Off => "0",
                    AlarmState::OnThres => "1",
                    AlarmState::OnStThres => "2",
                    AlarmState::OnLtThres => "3",
                }
            ))
        } else {
            None
        }
    }
}

impl Item for LogEntry {
    fn is_valid(&self) -> bool {
        self.valid == Self::VALID
    }

    fn max_size() -> usize {
        u8::MAX as usize
    }
}

// vim: ts=4 sw=4 expandtab
