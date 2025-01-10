use bittide::bittide::{BittideMessage, Fifo, Links};
use cortex_m::peripheral::syst::SystClkSource;
use heapless::Vec;
// TODO: should not really import from rp_pico but from the rp2040 crates
use rp_pico::{
    hal::pio::{Rx, Tx, SM0, SM1, SM2, SM3},
    pac::{PIO0, PIO1, SYST},
};

pub struct Rp2040Links {
    rxs: Rp2040Rxs,
    txs: Rp2040Txs,
}

impl Rp2040Links {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rx0: Rx<(PIO0, SM0)>,
        rx1: Rx<(PIO0, SM1)>,
        rx2: Rx<(PIO0, SM2)>,
        rx3: Rx<(PIO0, SM3)>,
        tx0: Tx<(PIO1, SM0)>,
        tx1: Tx<(PIO1, SM1)>,
        tx2: Tx<(PIO1, SM2)>,
        tx3: Tx<(PIO1, SM3)>,
    ) -> Self {
        Self {
            rxs: Rp2040Rxs::new(rx0, rx1, rx2, rx3),
            txs: Rp2040Txs::new(tx0, tx1, tx2, tx3),
        }
    }
}

impl Links<4> for Rp2040Links {
    fn write(&mut self, messages: [BittideMessage; 4]) {
        self.txs.write(messages);
    }

    fn read(&mut self) -> [Vec<BittideMessage, 4>; 4] {
        self.rxs.read()
    }

    fn active_fifos(&self) -> [bool; 4] {
        self.rxs.active_fifos()
    }
}

pub struct Rp2040Txs {
    tx0: Tx<(PIO1, SM0)>,
    tx1: Tx<(PIO1, SM1)>,
    tx2: Tx<(PIO1, SM2)>,
    tx3: Tx<(PIO1, SM3)>,
}

impl Rp2040Txs {
    pub fn new(
        tx0: Tx<(PIO1, SM0)>,
        tx1: Tx<(PIO1, SM1)>,
        tx2: Tx<(PIO1, SM2)>,
        tx3: Tx<(PIO1, SM3)>,
    ) -> Self {
        Self { tx0, tx1, tx2, tx3 }
    }

    fn write(&mut self, messages: [BittideMessage; 4]) {
        self.tx0.write(messages[0].serialize());
        self.tx1.write(messages[1].serialize());
        self.tx2.write(messages[2].serialize());
        self.tx3.write(messages[3].serialize());
    }
}

pub struct Rp2040Rxs {
    rx0: Rx<(PIO0, SM0)>,
    rx1: Rx<(PIO0, SM1)>,
    rx2: Rx<(PIO0, SM2)>,
    rx3: Rx<(PIO0, SM3)>,
    no_msg_counters: [usize; 4],
}

impl Rp2040Rxs {
    const NO_MESSAGE_LIMIT: usize = 3;

    pub fn new(
        rx0: Rx<(PIO0, SM0)>,
        rx1: Rx<(PIO0, SM1)>,
        rx2: Rx<(PIO0, SM2)>,
        rx3: Rx<(PIO0, SM3)>,
    ) -> Self {
        Self {
            rx0,
            rx1,
            rx2,
            rx3,
            no_msg_counters: [Self::NO_MESSAGE_LIMIT; 4],
        }
    }

    /// Read at most 4 values from each RX fifo.
    /// The FIFOs hold 4 values, and if a neighbor is driving them faster than this node is running,
    /// it's possible for there to be more than one value present. So read exactly four times every
    /// time the control algo runs to keep up with clocks up to 4x this node's frequency.
    /// Also adjusts the message such that the neighbor field shows what neighbor it came from.
    /// TODO: that adjustment has to be done on every impl of rxs..?
    fn read(&mut self) -> [Vec<BittideMessage, 4>; 4] {
        macro_rules! read {
            ($rx:ident, $fifo_id:expr) => {{
                let messages = (0..3)
                    .filter_map(|_| {
                        self.$rx.read().map(|w| {
                            let mut message = BittideMessage::deserialize(w);

                            if let BittideMessage::CommMessage { neighbor: _, data } = message {
                                message = BittideMessage::CommMessage {
                                    neighbor: $fifo_id,
                                    data,
                                }
                            }

                            message
                        })
                    })
                    .collect::<Vec<_, 4>>();

                if messages.is_empty() {
                    self.no_msg_counters[$fifo_id] += 1;
                } else {
                    self.no_msg_counters[$fifo_id] = 0;
                }

                messages
            }};
        }

        [read!(rx0, 0), read!(rx1, 1), read!(rx2, 2), read!(rx3, 3)]
    }

    /// Returns the amount of RX FIFO's that have seen messages on the last few runs.
    /// Necessary to determine setpoints automatically in networks where not every node has the same amount of neighbors.
    fn active_fifos(&self) -> [bool; 4] {
        let mut actives = [false; 4];

        self.no_msg_counters
            .iter()
            .map(|&counter| counter < Self::NO_MESSAGE_LIMIT)
            .enumerate()
            .for_each(|(i, b)| actives[i] = b);

        actives
    }
}

pub struct SioFifo(pub rp_pico::hal::sio::SioFifo);

impl Fifo for SioFifo {
    fn read(&mut self) -> Option<u32> {
        self.0.read()
    }

    fn write(&mut self, data: u32) {
        self.0.write(data);
    }
}

/// Requires a setup like this in the main to define what happens on the systick connection:
///
/// ```rust
/// static GLOBAL_CONTROL: Mutex<RefCell<Option<bittide_impls::boards::pico1_and_si5351::Control>>> = Mutex::new(RefCell::new(None));
///
/// #[exception]
/// fn SysTick() {
///     static mut CONTROL: Option<bittide_impls::boards::pico1_and_si5351::Control> = None;
///
///     if CONTROL.is_none() {
///         critical_section::with(|cs| {
///             let _ = CONTROL.insert(GLOBAL_CONTROL.borrow(cs).take().unwrap());
///         });
///     }
///
///     if let Some(control) = CONTROL {
///         control.interrupt();
///     }
/// }
/// ```
pub fn setup_interrupt(clocks_per_sync_word: u32, systick: &mut SYST) {
    systick.set_reload(clocks_per_sync_word - 1);
    systick.clear_current();
    systick.enable_counter();
    systick.set_clock_source(SystClkSource::Core);
    systick.enable_interrupt();
}
