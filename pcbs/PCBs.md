# Custom RP2040 based board

Custom board with RP2040 chip and a Si5351A clock generator module running off a crystal. The board boots on the ring buffer and configures the Si5351A to 10MHz before switching to the PLL.

Features:
- RP2040 as compute
- Si5351A clock generator on the PCB + crystal
- 4 GPIO blocks with:
    - 3 TX pins
    - 3 RX pins
    - Power distribution

To work out:
- Easy programming access? Seperate board with analog multiplexers to switch SWD lines?