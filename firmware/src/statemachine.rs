// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use crate::envsensor::EnvSensorResult;
use std::{collections::VecDeque, num::Saturating};

const MEAS_INTERVAL_MS: u32 = 100; // task_100ms
const MEAS_DURN_MS: u32 = 10_000;
const MEAS_LEN: usize = (MEAS_DURN_MS / MEAS_INTERVAL_MS) as usize;

// Alarm ON
const HUM_ALARM_ON_THRES: f32 = percent(70.0);
const D_HUM_ALARM_ON_THRES: f32 = percent(5.0);
// Alarm OFF
const OFF_COUNTER_THRES: u8 = 15;
const HUM_ALARM_OFF_THRES: f32 = percent(50.0);

const fn percent(val: f32) -> f32 {
    val / 100.0
}

const fn to_percent(val: f32) -> f32 {
    val * 100.0
}

pub struct StateMachine {
    meas: VecDeque<EnvSensorResult>,
    alarm: bool,
    filled: bool,
    off_counter: Saturating<u8>,
}

impl StateMachine {
    pub fn new() -> Self {
        StateMachine {
            meas: VecDeque::new(),
            alarm: false,
            filled: false,
            off_counter: Saturating(0),
        }
    }

    pub fn feed_env_100ms(&mut self, env: EnvSensorResult) {
        while self.meas.len() >= MEAS_LEN {
            self.meas.pop_front();
        }
        self.meas.push_back(env);
    }

    pub fn evaluate_1000ms(&mut self) {
        if !self.filled && self.meas.len() >= MEAS_LEN {
            self.filled = true;
            println!("Meas buffer filled, starting evaluation...");
        }
        if self.filled
            && let Some(meas_front) = self.meas.front()
            && let Some(meas_back) = self.meas.back()
        {
            let rel_hum = meas_back.rel_hum;
            let d_hum = rel_hum - meas_front.rel_hum;

            println!(
                "hum: {:.1} % / {:.1} %, d_hum: {:.1} % / {:.1} %, alarm: {}, off_counter: {} / {}",
                to_percent(rel_hum),
                to_percent(HUM_ALARM_ON_THRES),
                to_percent(d_hum),
                to_percent(D_HUM_ALARM_ON_THRES),
                self.alarm,
                self.off_counter.0,
                OFF_COUNTER_THRES
            );

            if rel_hum > HUM_ALARM_ON_THRES || d_hum >= D_HUM_ALARM_ON_THRES {
                self.alarm = true;
                self.off_counter = Saturating(0);
            } else if rel_hum < HUM_ALARM_OFF_THRES {
                self.off_counter += 1;
                if self.off_counter >= Saturating(OFF_COUNTER_THRES) {
                    self.alarm = false;
                }
            }
        }
    }

    pub fn alarm_active(&self) -> bool {
        self.alarm
    }
}

// vim: ts=4 sw=4 expandtab
