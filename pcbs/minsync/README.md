# `minsync`

A minimal testbed for a RP2040-based board with an Si5351 as a clock source instead of the usual 12MHz crystal. 

# IO:

- Four clusters of six GPIO pins and 5V + GND power pins to allow easy chaining of the board.
- Headers for SWD flashing
- A bootsel button
- Testpoint for a gpout pin for debugging internal clocks

# Components

Si5351A (0.77): https://jlcpcb.com/partdetail/skyworks_siliconLabs-SI5351A_BGTR/C504891 
25 MHz crystal (0.25): https://jlcpcb.com/partdetail/Ndk-NX3225GA_25_000M_STD_CRG2/C1985619
RP2040 (0.95): https://jlcpcb.com/partdetail/RaspberryPi-RP2040/C2040
Voltage regulator (0.51): https://jlcpcb.com/partdetail/Onsemi-NCP1117DT33G/C154606
quad SPI flash (0.51): https://jlcpcb.com/partdetail/WinbondElec-W25Q128JVSIQ/C97521
Button for reset (0.57): https://jlcpcb.com/partdetail/ck-KMR221GLFS/C72443
Male headers x4 (0.60): https://jlcpcb.com/partdetail/225573-A2005WV2x5P/C225290