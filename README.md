# Wetdet - Monitor air humidity and give alarm for sudden rise in humidity

Wetdet is a simple tool that monitors air humidity levels and triggers an alarm if there is a sudden rise in humidity or if relative humidity exceeds a certain threshold.

Alarm trigger conditions:

- Air delta humidity change: `hum(t) - hum(t-300s) > 1.5 %rel`
- or air humidity: `> 92 %rel`

Alarm shutoff conditions:

- Air humidity: `< 50 %rel` for 120 seconds

These parameters can all be adjusted in the file `firmware/src/config.rs`.

## Hardware

- ESP32-WROOM microcontroller
- BME280 sensor for measuring humidity
- Digital alarm output signal (GPIO)

An external alarm module with speaker or buzzer or any other way to react to the alarm signal can be connected to the digital output pin.

## License / Copyright

Copyright Michael Büsch <m@bues.ch>

SPDX-License-Identifier: Apache-2.0 OR MIT
