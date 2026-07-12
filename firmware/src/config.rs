// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

use crate::util::percent;

// General

/// Whether to print the state of the system to the serial console.
pub const PRINT_STATE: bool = true;

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

// vim: ts=4 sw=4 expandtab
