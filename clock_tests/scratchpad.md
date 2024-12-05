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