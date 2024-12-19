# `minsync`

A minimal testbed for a RP2040-based board with an Si5351 as a clock source instead of the usual 12MHz crystal. 

# IO:

- Four clusters of six GPIO pins and 5V + GND power pins to allow easy chaining of the board.
- Headers for SWD flashing
- A bootsel button

# Components

To be formalized in sheet

Button: https://jlcpcb.com/partdetail/Alpsalpine-SKHMQLE010/C139767
Si5351A (0.77): https://jlcpcb.com/partdetail/skyworks_siliconLabs-SI5351A_BGTR/C504891 
25MHz crystal (0.32): https://jlcpcb.com/partdetail/AbraconLlc-ABM8_25_000MHZ_B2T/C596899
RP2040 (0.95): https://jlcpcb.com/partdetail/RaspberryPi-RP2040/C2040
Voltage regulator (0.51): https://jlcpcb.com/partdetail/Onsemi-NCP1117DT33G/C154606
quad SPI flash (0.51): https://jlcpcb.com/partdetail/WinbondElec-W25Q128JVSIQ/C97521