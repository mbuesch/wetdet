// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use crate::{
    config::{
        D_HUM_LT_ALARM_ON_THRES, D_HUM_ST_ALARM_ON_THRES, HUM_ALARM_OFF_THRES, HUM_ALARM_ON_THRES,
        MEAS_LEN_S, MEAS_LT_FACT, OFF_SEC_THRES, PRINT_STATE,
    },
    envsensor::EnvSensorResult,
    util::{min, to_percent},
};
use heapless::Deque;
use logentry::AlarmState;

/// Measurement interval (in milliseconds).
const MEAS_INTERVAL_MS: u32 = 1_000; // task_1s
/// Measurement buffer length (in number of entries).
const MEAS_LEN: usize = ((MEAS_LEN_S * 1_000).div_ceil(MEAS_INTERVAL_MS)) as usize;
/// Minimum number of measurements in the buffer before evaluation starts.
const MEAS_MIN_FILL: usize = min(MEAS_LEN, 10);

pub struct StateMachine {
    meas_st: Deque<EnvSensorResult, MEAS_LEN>,
    meas_lt: Deque<EnvSensorResult, MEAS_LEN>,
    meas_lt_count: usize,
    alarm: AlarmState,
    filled: bool,
    off_sec: u32,
}

impl StateMachine {
    pub fn new() -> Self {
        if PRINT_STATE {
            println!(
                "Humidity alarm ON threshold: {:.1} %rel, d_hum: {:.1} %rel (st), {:.1} %rel (lt)",
                to_percent(HUM_ALARM_ON_THRES),
                to_percent(D_HUM_ST_ALARM_ON_THRES),
                to_percent(D_HUM_LT_ALARM_ON_THRES)
            );
            println!(
                "Humidity alarm OFF threshold: {:.1} %rel, off_sec: {} s",
                to_percent(HUM_ALARM_OFF_THRES),
                OFF_SEC_THRES
            );
        }
        StateMachine {
            meas_st: Deque::new(),
            meas_lt: Deque::new(),
            meas_lt_count: 0,
            alarm: AlarmState::Off,
            filled: false,
            off_sec: 0,
        }
    }

    fn push_meas(meas: &mut Deque<EnvSensorResult, MEAS_LEN>, env: &EnvSensorResult) {
        while meas.len() >= MEAS_LEN {
            meas.pop_front();
        }
        meas.push_back(env.clone()).expect("Deque::push_back failed");
    }

    pub fn feed_env_1000ms(&mut self, env: &EnvSensorResult) {
        Self::push_meas(&mut self.meas_st, env);

        if self.meas_lt_count == 0 {
            Self::push_meas(&mut self.meas_lt, env);
        }
        self.meas_lt_count = (self.meas_lt_count + 1) % MEAS_LT_FACT;
    }

    pub fn evaluate_1000ms(&mut self) {
        if !self.filled && self.meas_st.len() >= MEAS_MIN_FILL {
            self.filled = true;
            if PRINT_STATE {
                println!("Meas buffer filled, starting evaluation...");
            }
        }
        if self.filled
            && let Some(meas_st_front) = self.meas_st.front()
            && let Some(meas_st_back) = self.meas_st.back()
            && let Some(meas_lt_front) = self.meas_lt.front()
            && let Some(meas_lt_back) = self.meas_lt.back()
        {
            let rel_hum = meas_st_back.rel_hum;
            let d_hum_st = meas_st_back.rel_hum - meas_st_front.rel_hum;
            let d_hum_lt = meas_lt_back.rel_hum - meas_lt_front.rel_hum;

            if PRINT_STATE {
                println!(
                    "\nhum: {:.1} %rel / {:.1} %rel, \
                    d_hum_st: {:.1}/{:.1} %rel, \
                    d_hum_lt: {:.1}/{:.1} %rel, \
                    {:?}, off_sec: {}/{} s",
                    to_percent(rel_hum),
                    to_percent(HUM_ALARM_ON_THRES),
                    to_percent(d_hum_st),
                    to_percent(D_HUM_ST_ALARM_ON_THRES),
                    to_percent(d_hum_lt),
                    to_percent(D_HUM_LT_ALARM_ON_THRES),
                    self.alarm,
                    self.off_sec,
                    OFF_SEC_THRES
                );
            }

            if rel_hum > HUM_ALARM_ON_THRES {
                if self.alarm == AlarmState::Off {
                    println!(
                        "Trigger: Humidity alarm ON (rel_hum: {:.1} %rel)",
                        to_percent(rel_hum)
                    );
                }
                self.alarm = AlarmState::OnThres;
                self.off_sec = 0;
            } else if d_hum_st >= D_HUM_ST_ALARM_ON_THRES {
                if self.alarm == AlarmState::Off {
                    println!(
                        "Trigger: Humidity alarm ON (d_hum: {:.1} %rel short-term)",
                        to_percent(d_hum_st)
                    );
                }
                self.alarm = AlarmState::OnStThres;
                self.off_sec = 0;
            } else if d_hum_lt >= D_HUM_LT_ALARM_ON_THRES {
                if self.alarm == AlarmState::Off {
                    println!(
                        "Trigger: Humidity alarm ON (d_hum: {:.1} %rel long-term)",
                        to_percent(d_hum_lt)
                    );
                }
                self.alarm = AlarmState::OnLtThres;
                self.off_sec = 0;
            } else if rel_hum < HUM_ALARM_OFF_THRES {
                if self.off_sec >= OFF_SEC_THRES {
                    self.alarm = AlarmState::Off;
                } else {
                    self.off_sec += 1;
                }
            } else {
                self.off_sec = 0;
            }
        }
    }

    pub fn alarm_state(&self) -> AlarmState {
        self.alarm
    }
}

// vim: ts=4 sw=4 expandtab
