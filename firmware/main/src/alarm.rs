// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use crate::{
    config::{ALARM_OFF_TIME, ALARM_ON_TIME, PRINT_STATE},
    esp_idf::hal::gpio::{AnyIOPin, Level, Output, PinDriver},
};

pub const fn alarm_time_ms(ms: u32) -> u32 {
    ms.div_ceil(100)
}

pub struct Alarm<'a> {
    active: bool,
    pin: PinDriver<'a, Output>,
    count: u32,
}

impl<'a> Alarm<'a> {
    pub fn new(pin: AnyIOPin<'a>) -> Self {
        let mut pin = PinDriver::output(pin).expect("Alarm pin init failed");
        pin.set_low().expect("Alarm pin set low failed");
        Alarm {
            active: false,
            pin,
            count: 0,
        }
    }

    pub fn activate(&mut self, active: bool) {
        if self.active != active && PRINT_STATE {
            println!("Alarm: {}", if active { "ON" } else { "OFF" });
        }
        self.active = active;
    }

    pub fn run_100ms(&mut self) {
        let level;
        if self.active {
            if self.count < ALARM_ON_TIME || ALARM_OFF_TIME == 0 {
                level = Level::High;
            } else {
                level = Level::Low;
            }
            self.count = (self.count + 1) % (ALARM_ON_TIME + ALARM_OFF_TIME);
        } else {
            level = Level::Low;
            self.count = 0;
        }
        self.pin
            .set_level(level)
            .expect("Alarm pin set level failed");
    }
}

// vim: ts=4 sw=4 expandtab
