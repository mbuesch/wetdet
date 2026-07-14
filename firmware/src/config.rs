// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use crate::{alarm::alarm_time_ms, util::percent};

// General

/// Whether to print the state of the system to the serial console.
pub const PRINT_STATE: bool = true;

// Measurement

/// Measurement buffer length (in seconds).
/// This many measurements are stored in the buffer to evaluate the alarm state.
/// D_HUM is calculated based on the first and last measurement in the buffer.
pub const MEAS_LEN_S: u32 = 120;

// Alarm ON

/// Relative air humidity threshold to trigger the alarm.
pub const HUM_ALARM_ON_THRES: f32 = percent(70.0);
/// Sudden change in air humidity to trigger the alarm.
pub const D_HUM_ALARM_ON_THRES: f32 = percent(5.0);

// Alarm OFF

/// Relative air humidity threshold to turn off the alarm.
pub const HUM_ALARM_OFF_THRES: f32 = percent(50.0);
/// Time (in seconds) the relative air humidity must be below the threshold to turn off the alarm.
pub const OFF_SEC_THRES: u32 = 15;

// Alarm PWM

/// Time the digital output is high when the alarm is active.
pub const ALARM_ON_TIME: u32 = alarm_time_ms(700);
/// Time the digital output is low when the alarm is active.
/// If set to 0, the output will be high all the time when the alarm is active.
pub const ALARM_OFF_TIME: u32 = alarm_time_ms(3000);

// vim: ts=4 sw=4 expandtab
