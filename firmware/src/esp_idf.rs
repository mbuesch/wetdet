// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

#![allow(unused_imports)]

#[cfg(not(feature = "hw"))]
pub use dummy_esp_idf_hal as hal;
#[cfg(feature = "hw")]
pub use esp_idf_hal as hal;

#[cfg(not(feature = "hw"))]
pub use dummy_esp_idf_svc as svc;
#[cfg(feature = "hw")]
pub use esp_idf_svc as svc;

#[cfg(not(feature = "hw"))]
pub use dummy_esp_idf_sys as sys;
#[cfg(feature = "hw")]
pub use esp_idf_sys as sys;

// vim: ts=4 sw=4 expandtab
