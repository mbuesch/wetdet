# Wetdet - Monitor air humidity and give alarm for sudden rise in humidity

Wetdet is a simple tool that monitors air humidity levels and triggers an alarm if there is a sudden rise in humidity or if relative humidity exceeds a certain threshold.

Alarm trigger conditions:

- Air humidity: `> 80 %rel`
- or delta humidity change: `hum(t) - hum(t-10s) > 5 %rel`

## Hardware

- ESP32 microcontroller
- BME280 sensor for measuring humidity
- Alarm module with speaker or buzzer, controlled by GPIO alarm on/off

## License / Copyright

Copyright Michael Büsch <m@bues.ch>

SPDX-License-Identifier: Apache-2.0 OR MIT
