// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

pub struct ErrorDeb<const RATIO: u8, const LIMIT: u8> {
    count: u8,
}

impl<const RATIO: u8, const LIMIT: u8> ErrorDeb<RATIO, LIMIT> {
    pub const fn new() -> Self {
        Self { count: 0 }
    }

    pub fn is_ok(&self) -> bool {
        self.count < LIMIT * RATIO
    }

    pub fn error(&mut self) -> bool {
        self.count = self.count.saturating_add(RATIO);
        !self.is_ok()
    }

    pub fn ok(&mut self) -> bool {
        self.count = self.count.saturating_sub(1);
        self.is_ok()
    }
}

// vim: ts=4 sw=4 expandtab
