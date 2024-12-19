# Custom RP2040 based board

Custom board with RP2040 chip and a Si5351A clock generator module running off a crystal. The board boots on the ring buffer and configures the Si5351A to 12MHz before switching to the PLL.

Features:
- RP2040 as compute + quad SPI flash + voltage regulator
- Si5351A clock generator on the PCB + crystal
    - Clock 0 connected to XIN (accepts a CMOS clock), and goes into the PLL
    - Clock 1 connected to CLKSRC_GPIN0 as a backup
- 4 GPIO blocks with:
    - 3 TX pins
    - 3 RX pins
    - Power distribution

To work out:
- Easy programming access? Seperate board with analog multiplexers to switch SWD lines? Shift registers + analog multiplexers on the board..?

Daisy Chained SWD:
- use two analog multiplexers to switch the two SWD lines between going to the next pico or this pico. 
- Control the analog multiplexer using a single bit register (e.g. https://jlcpcb.com/partdetail/Nexperia-74LVC1G74GT115/C548415) configured as a shift register. 
This setup allows daisy chaining using four wires (two for the register, clock and data, and two for the SWD interface itself.)