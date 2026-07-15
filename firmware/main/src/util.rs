// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

pub const fn percent(val: f32) -> f32 {
    val / 100.0
}

pub const fn to_percent(val: f32) -> f32 {
    val * 100.0
}

pub const fn min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

// vim: ts=4 sw=4 expandtab
