# Physical Build

2 minsync v0.2 boards connected to eachother only via the north <-> south connection. On these boards the SI5351 can make minute changes to the clock frequency that the system runs on. An extra RPi pico is used as a programmer using [this flashing software fork](https://github.com/PietPtr/debugprobe-variable-swdio). The boards are connected to GPIO4 and 5 of the programmer Pico.