// -*- coding: utf-8 -*-

use crate::{
    errordeb::ErrorDeb,
    esp_idf::hal::{
        gpio::AnyIOPin,
        i2c::{I2c, I2cConfig, I2cDriver},
        units::KiloHertz,
    },
};
use bme280_multibus::{
    Bme280, Config, CtrlMeas, Filter, Mode, Oversampling, Settings, Standby,
    i2c1::{Address, Bme280Bus},
};

#[derive(Clone, Debug, Default)]
pub struct EnvSensorResult {
    pub temp_c: f32,
    pub pres_hpa: f32,
    pub rel_hum: f32,
}

const NUM_SKIP: u8 = 2;

const TEMP_C_LIMITS: (f32, f32) = (-40.0, 80.0);
const PRES_HPA_LIMITS: (f32, f32) = (900.0, 1100.0);
const REL_HUM_LIMITS: (f32, f32) = (0.0, 1.0);

impl EnvSensorResult {
    #[allow(dead_code)]
    pub fn print(&self) {
        println!(
            "Environment: t = {}.{} *C, p = {} hPa, h = {} %",
            self.temp_c as i32,
            ((self.temp_c * 10.0) % 10.0) as i32,
            self.pres_hpa.round() as i32,
            (self.rel_hum * 100.0).round() as i32
        );
    }
}

pub struct EnvSensor<'a> {
    bme: Bme280<Bme280Bus<I2cDriver<'a>>>,
    error: ErrorDeb<3, 5>,
    skip: u8,
}

impl<'a> EnvSensor<'a> {
    pub fn new(i2c: impl I2c + 'a, sda: AnyIOPin<'a>, scl: AnyIOPin<'a>) -> Self {
        let i2c_config = I2cConfig::new()
            .baudrate(KiloHertz(10).into())
            .sda_enable_pullup(false)
            .scl_enable_pullup(false);
        let i2cdrv = I2cDriver::new(i2c, sda, scl, &i2c_config).expect("BME280 I2C init failed");

        let mut bme = Bme280::from_i2c1(i2cdrv, Address::SdoGnd).expect("BME280 init failed");

        bme.reset().expect("BME280 reset failed");

        let config = Config::RESET
            .set_filter(Filter::Off)
            .set_standby_time(Standby::Micros62500);

        let ctrl_meas = CtrlMeas::RESET
            .set_mode(Mode::Normal)
            .set_osrs_p(Oversampling::X16)
            .set_osrs_t(Oversampling::X16);

        let ctrl_hum = Oversampling::X16;

        bme.settings(&Settings {
            config,
            ctrl_meas,
            ctrl_hum,
        })
        .expect("BME280 settings failed");

        Self {
            bme,
            error: ErrorDeb::new(),
            skip: NUM_SKIP,
        }
    }

    pub fn read(&mut self) -> Option<EnvSensorResult> {
        if self.skip > 0 {
            self.skip -= 1;
            return None;
        }

        let sample = match self.bme.sample() {
            Ok(sample) => sample,
            Err(e) => {
                eprintln!("bme: Sample read error: {e:?}");
                if self.error.error() {
                    panic!("bme: Sample read error panic.");
                }
                if e == bme280_multibus::Error::Sample {
                    self.skip = NUM_SKIP;
                }
                return None;
            }
        };

        let res = EnvSensorResult {
            temp_c: sample.temperature,
            pres_hpa: sample.pressure / 100.0,
            rel_hum: sample.humidity / 100.0,
        };

        if res.temp_c < TEMP_C_LIMITS.0 || res.temp_c > TEMP_C_LIMITS.1 {
            eprintln!("bme: Temperature sanity check failed: {}", res.temp_c);
            if self.error.error() {
                panic!("bme: temp_c sanity check panic.");
            }
            return None;
        }
        if res.pres_hpa < PRES_HPA_LIMITS.0 || res.pres_hpa > PRES_HPA_LIMITS.1 {
            eprintln!("bme: Pressure sanity check failed: {}", res.pres_hpa);
            if self.error.error() {
                panic!("bme: pres_hpa sanity check panic.");
            }
            return None;
        }
        if res.rel_hum < REL_HUM_LIMITS.0 || res.rel_hum > REL_HUM_LIMITS.1 {
            eprintln!("bme: Humidity sanity check failed: {}", res.rel_hum);
            if self.error.error() {
                panic!("bme: rel_hum sanity check panic.");
            }
            return None;
        }

        self.error.ok();
        Some(res)
    }
}

// vim: ts=4 sw=4 expandtab
