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
mod measlog;
mod statemachine;
mod util;

use crate::{
    alarm::Alarm,
    config::PRINT_STATE,
    envsensor::EnvSensor,
    esp_idf::{
        hal::{
            self as hal,
            gpio::AnyIOPin,
            peripherals::Peripherals,
            spi::{self, SpiDriver},
            uart::UartDriver,
            units::Hertz,
        },
        sys::{bootloader_random_enable, esp_random},
    },
    measlog::MeasLog,
    statemachine::StateMachine,
};
use logentry::{LogEntry, AlarmState};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, Ordering},
};

struct System<'a> {
    boot_id: u32,
    #[allow(dead_code)]
    prog_uart: Mutex<UartDriver<'a>>,
    envsensor: Mutex<EnvSensor<'a>>,
    statemachine: Mutex<StateMachine>,
    alarm: Mutex<Alarm<'a>>,
    measlog: Mutex<MeasLog<'a>>,
    measlog_serial: AtomicU32,
}

impl<'a> System<'a> {
    fn new(
        boot_id: u32,
        prog_uart: UartDriver<'a>,
        envsensor: EnvSensor<'a>,
        alarm: Alarm<'a>,
        measlog: MeasLog<'a>,
    ) -> Self {
        System {
            boot_id,
            prog_uart: Mutex::new(prog_uart),
            envsensor: Mutex::new(envsensor),
            statemachine: Mutex::new(StateMachine::new()),
            alarm: Mutex::new(alarm),
            measlog: Mutex::new(measlog),
            measlog_serial: AtomicU32::new(0),
        }
    }
}

timeslice::define_sched! {
    name: sched_main,
    num_objs: 1,
    tasks: {
        { name: task_100ms, period: 100 ms,    cpu: 0, prio: 9, stack: 8 kiB },
        { name: task_1s,    period: 1_000 ms,  cpu: 0, prio: 8, stack: 8 kiB },
        { name: task_5s,    period: 5_000 ms,  cpu: 1, prio: 7, stack: 8 kiB },
        { name: task_60s,   period: 60_000 ms, cpu: 1, prio: 6, stack: 8 kiB },
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
        let alarm_state = {
            let mut statemachine = self.statemachine.lock().unwrap();
            if let Some(env) = &env {
                statemachine.feed_env_1000ms(env);
            }
            statemachine.evaluate_1000ms();
            statemachine.alarm_state()
        };
        {
            let mut alarm = self.alarm.lock().unwrap();
            alarm.activate(alarm_state != AlarmState::Off);
        }
        if let Some(env) = &env {
            let log = LogEntry::new(
                self.boot_id,
                self.measlog_serial.fetch_add(1, Ordering::Relaxed),
                env.temp_c,
                env.pres_hpa,
                env.rel_hum,
                alarm_state,
            );
            let mut measlog = self.measlog.lock().unwrap();
            measlog.push_entry(log);
        }
    }

    fn task_5s(&self) {
        sched_main::rt_print();
    }

    fn task_60s(&self) {
        {
            let mut measlog = self.measlog.lock().unwrap();
            measlog.commit();
        }
    }
}

fn main() {
    let boot_id = unsafe {
        bootloader_random_enable();
        esp_random()
    };

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

    let spi_config = spi::config::DriverConfig::new();
    let spi2_drv = Arc::new(
        SpiDriver::new(
            dp.spi2,
            dp.pins.gpio18,
            dp.pins.gpio23,
            Some(dp.pins.gpio19),
            &spi_config,
        )
        .unwrap(),
    );

    let envsensor = EnvSensor::new(dp.i2c0, dp.pins.gpio26.into(), dp.pins.gpio25.into());

    let alarm = Alarm::new(dp.pins.gpio12.into());

    let measlog = MeasLog::new(spi2_drv, dp.pins.gpio5.degrade_output());

    println!("Boot ID: {boot_id:08x}");
    let system = Arc::new(System::new(boot_id, prog_uart, envsensor, alarm, measlog));
    sched_main::init([system]);
    sched_main::rt_enable(PRINT_STATE);
}

// vim: ts=4 sw=4 expandtab
