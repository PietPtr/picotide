# SCRATCHPAD

PIO state machines are connected to a clock and have a clock divisor:
```
        .clock_divisor_fixed_point(MCLK_CLOCKDIV_INT, MCLK_CLOCKDIV_FRAC)
```
The PIO is connected to the sys clk, so by setting a fixed divisor of the PIO we can subdivide the system clock, and changing the system clock directly influences the speed that the PIO program runs at.

There is 1 32-bit, 4 word FIFO connected to each of the two PIO blocks. 4 words is unsuitable for bittide, so we'll need a virtual FIFO (ring buffer) as well. 

## Runtime

(async). PIOs send/retrieve the next sync/data word and put it in their FIFO.
1. Pull sync word from pio fifo and put in ring buffer.
2. execute control algorithm
3. run user logic (On other core: can put words marked as user words on the SIO FIFO)

So the clock divisors on the PIO determines how much time there is for 1/2/3.

## User logic

**As producer consumer**: On the other core user logic can run that reads from / writes to the SIO FIFO. All these user cores will run at the same clock speed due to the control algorithm, but if everything is a producer/consumer anyway and expects to sometimes wait for a fifo to clear/fill that's not of much use.

**As a tightly syncronized core**: If compiling on the _assumption_ that the user cores run at the same logical frequency is possible interesting scheduling is possible. It probably means that some user logic should be nop-padded. This is possible by setting a strict cycle budget per sync/data word coming in from the PIOs. The user logic should be driven / run in an environment that counts cycles. That's possible with the `syst_cvr` counter.

## Clock experiments results

The PLL is easy to control through its registers (specifically fbdiv), and these can be changed at runtime to change the frequency. The resolution is dependent on how much the ref clock is divided. Using settings to get a 100MHz clock, incrementing the fbdiv by 1 results in a 1MHz change. 

For more precise clock control, consider an Si5351A (0.77 cents) on a custom PCB.

## Minimum cost sync pico

Minimum cost of a pico like board with an Si5351A and RP2040 and required components

Expensive components:
Si5351A (0.77): https://jlcpcb.com/partdetail/skyworks_siliconLabs-SI5351A_BGTR/C504891 
+ crystal (0.32): https://jlcpcb.com/partdetail/AbraconLlc-ABM8_25_000MHZ_B2T/C596899
RP2040 (0.95): https://jlcpcb.com/partdetail/RaspberryPi-RP2040/C2040
+ crystal (0.45): https://jlcpcb.com/partdetail/AbraconLlc-ABM8_272T3/C20625731 likely unnecessary: run of ring oscillator until clock from Si5351 is available
Voltage regulator (0.51): https://jlcpcb.com/partdetail/Onsemi-NCP1117DT33G/C154606
quad SPI flash (0.51): https://jlcpcb.com/partdetail/WinbondElec-W25Q128JVSIQ/C97521
= 3.51

Then other components come in like capacitors/resistors, and PCBA. 
