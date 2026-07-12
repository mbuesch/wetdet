// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use crate::{
    config::{D_HUM_ALARM_ON_THRES, HUM_ALARM_OFF_THRES, HUM_ALARM_ON_THRES, OFF_SEC_THRES},
    envsensor::EnvSensorResult,
    util::to_percent,
};
use std::collections::VecDeque;

/// Measurement interval (in milliseconds).
const MEAS_INTERVAL_MS: u32 = 1_000; // task_1s
/// Measurement buffer length (in milliseconds).
const MEAS_LEN_MS: u32 = 10_000;
/// Measurement buffer length (in number of entries).
const MEAS_LEN: usize = (MEAS_LEN_MS / MEAS_INTERVAL_MS) as usize;

pub struct StateMachine {
    meas: VecDeque<EnvSensorResult>,
    alarm: bool,
    filled: bool,
    off_sec: u32,
}

impl StateMachine {
    pub fn new() -> Self {
        println!(
            "Humidity alarm ON threshold: {:.1} %rel, d_hum: {:.1} %rel",
            to_percent(HUM_ALARM_ON_THRES),
            to_percent(D_HUM_ALARM_ON_THRES)
        );
        println!(
            "Humidity alarm OFF threshold: {:.1} %rel, off_sec: {} s",
            to_percent(HUM_ALARM_OFF_THRES),
            OFF_SEC_THRES
        );
        StateMachine {
            meas: VecDeque::new(),
            alarm: false,
            filled: false,
            off_sec: 0,
        }
    }

    pub fn feed_env_1000ms(&mut self, env: EnvSensorResult) {
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
                "\nhum: {:.1} %rel / {:.1} %rel, d_hum: {:.1} %rel / {:.1} %rel, alarm: {}, off_sec: {} s / {} s",
                to_percent(rel_hum),
                to_percent(HUM_ALARM_ON_THRES),
                to_percent(d_hum),
                to_percent(D_HUM_ALARM_ON_THRES),
                self.alarm,
                self.off_sec,
                OFF_SEC_THRES
            );

            if rel_hum > HUM_ALARM_ON_THRES || d_hum >= D_HUM_ALARM_ON_THRES {
                self.alarm = true;
                self.off_sec = 0;
            } else if rel_hum < HUM_ALARM_OFF_THRES {
                if self.off_sec >= OFF_SEC_THRES {
                    self.alarm = false;
                } else {
                    self.off_sec += 1;
                }
            }
        }
    }

    pub fn alarm_active(&self) -> bool {
        self.alarm
    }
}

// vim: ts=4 sw=4 expandtab
