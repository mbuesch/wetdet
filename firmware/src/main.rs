// -*- coding: utf-8 -*-
//
// Copyright Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT
//

#![cfg_attr(not(feature = "hw"), allow(dead_code))]

mod alarm;
mod config;
mod envsensor;
mod errordeb;
mod esp_idf;
mod statemachine;
mod util;

use crate::{
    alarm::Alarm,
    config::PRINT_STATE,
    envsensor::EnvSensor,
    esp_idf::{
        hal::{
            self as hal, gpio::AnyIOPin, peripherals::Peripherals, uart::UartDriver, units::Hertz,
        },
        sys::bootloader_random_enable,
    },
    statemachine::StateMachine,
};
use std::sync::{Arc, Mutex};

struct System<'a> {
    #[allow(dead_code)]
    prog_uart: Mutex<UartDriver<'a>>,
    envsensor: Mutex<EnvSensor<'a>>,
    statemachine: Mutex<StateMachine>,
    alarm: Mutex<Alarm<'a>>,
}

impl<'a> System<'a> {
    fn new(prog_uart: UartDriver<'a>, envsensor: EnvSensor<'a>, alarm: Alarm<'a>) -> Self {
        System {
            prog_uart: Mutex::new(prog_uart),
            envsensor: Mutex::new(envsensor),
            statemachine: Mutex::new(StateMachine::new()),
            alarm: Mutex::new(alarm),
        }
    }
}

timeslice::define_sched! {
    name: sched_main,
    num_objs: 1,
    tasks: {
        { name: task_100ms, period: 100 ms,   cpu: 0, prio: 9, stack: 8 kiB },
        { name: task_1s,    period: 1_000 ms, cpu: 0, prio: 8, stack: 8 kiB },
        { name: task_5s,    period: 5_000 ms, cpu: 0, prio: 7, stack: 8 kiB },
    },
}

impl<'a> sched_main::Ops for System<'a> {
    fn task_100ms(&self) {
        {
            let mut alarm = self.alarm.lock().unwrap();
            alarm.run_100ms();
        }
    }

    fn task_1s(&self) {
        let env = {
            let mut envsensor = self.envsensor.lock().unwrap();
            envsensor.read()
        };
        let alarm_active = {
            let mut statemachine = self.statemachine.lock().unwrap();
            if let Some(env) = env {
                statemachine.feed_env_1000ms(env);
            }
            statemachine.evaluate_1000ms();
            statemachine.alarm_active()
        };
        {
            let mut alarm = self.alarm.lock().unwrap();
            alarm.activate(alarm_active);
        }
    }

    fn task_5s(&self) {
        sched_main::rt_print();
    }
}

fn main() {
    unsafe {
        bootloader_random_enable();
    }

    let dp = Peripherals::take().expect("Peripherals::take() failed.");

    let config = hal::uart::config::Config::default().baudrate(Hertz(115_200));
    let prog_uart: hal::uart::UartDriver = hal::uart::UartDriver::new(
        dp.uart0,
        dp.pins.gpio1,
        dp.pins.gpio3,
        AnyIOPin::none(),
        AnyIOPin::none(),
        &config,
    )
    .expect("Failed to initialize programmer port.");

    let envsensor = EnvSensor::new(dp.i2c0, dp.pins.gpio19.into(), dp.pins.gpio18.into());

    let alarm = Alarm::new(dp.pins.gpio12.into());

    if PRINT_STATE {
        println!("Starting scheduler...");
    }
    let system = Arc::new(System::new(prog_uart, envsensor, alarm));
    sched_main::init([system]);
    sched_main::rt_enable(PRINT_STATE);
}

// vim: ts=4 sw=4 expandtab
