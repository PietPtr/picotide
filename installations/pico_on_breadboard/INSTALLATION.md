# Physical Build

Software written for 3 RPi pico's on breadboards without any eternal hardware, so they use the fbdiv controller to sync their clocks. They are connected via 6 jumper wires per pin. An extra RPi pico is used as a programmer using [this flashing software fork](https://github.com/PietPtr/debugprobe-variable-swdio), and each of the picos running the network is connected over SWD to this pico, each with their SWDIO connected to a different pin, using pins 3, 4, and 5 on the debugger Pico.

The picos are connected in either a line or a triangle topology.