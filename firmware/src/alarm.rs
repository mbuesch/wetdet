// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use crate::esp_idf::hal::gpio::{AnyIOPin, Level, Output, PinDriver};

pub struct Alarm<'a> {
    active: bool,
    pin: PinDriver<'a, Output>,
    count: u8,
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
        if self.active != active {
            println!("Alarm: {}", if active { "ON" } else { "OFF" });
        }
        self.active = active;
    }

    pub fn run_100ms(&mut self) {
        let mut level = Level::Low;
        if self.active {
            if self.count <= 6 {
                level = Level::High;
                self.count += 1;
            } else if self.count <= 35 {
                self.count += 1;
            } else {
                self.count = 0;
            }
        } else {
            self.count = 0;
        }
        self.pin
            .set_level(level)
            .expect("Alarm pin set level failed");
    }
}

// vim: ts=4 sw=4 expandtab
