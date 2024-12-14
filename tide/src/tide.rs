use controllers::controller::FrequencyController;
use heapless::{Deque, Vec};
use rp_pico::{
    hal::{
        pio::{Rx, Tx, SM0, SM1, SM2, SM3},
        sio::SioFifo,
    },
    pac::{PIO0, PIO1},
};

/// Generic over the frequency controller F, the amount of neighbors N
/// and the buffer size B.
/// TODO: Generic N does not fully work as Rxs/Txs are hardcoded to size 4.
/// TODO: make this a library
pub struct TideChannelControl<F, const N: usize, const B: usize> {
    frequency_controller: F,
    rxs: Rxs,
    txs: Txs,
    sio_fifo: SioFifo,
    tide_fifos: [TideFifo<B>; N],
}

impl<F, const N: usize, const B: usize> TideChannelControl<F, N, B>
where
    F: FrequencyController<N, B>,
{
    pub fn new(
        frequency_controller: F,
        rxs: Rxs,
        txs: Txs,
        sio_fifo: SioFifo,
        tide_fifos: [TideFifo<B>; N],
    ) -> Self {
        Self {
            frequency_controller,
            rxs,
            txs,
            sio_fifo,
            tide_fifos,
        }
    }

    /// All the logic to execute on a scheduled basis.
    /// This function must be called _exactly_ every `CLOCKS_PER_SYNC_WORD` system clock cycles.
    /// All clocks should be set up such that the execution of this function takes fewer clocks than that
    /// for its worst case execution path otherwise it cannot finish.
    pub fn interrupt(&mut self) {
        // Read user data from SIO FIFO
        let user_word = self.sio_fifo.read();

        // Send words on channel
        let mut messages = [TideMessage::SyncMessage; 4];

        if let Some(message) = user_word.map(TideMessage::deserialize) {
            match message {
                TideMessage::SyncMessage => panic!("unexpected"),
                TideMessage::CommMessage { neighbor, data: _ } => {
                    let neighbor = neighbor as usize;
                    if (neighbor) < N {
                        messages[neighbor] = message;
                    } else {
                        panic!("Invalid neigbor selected");
                    }
                }
            }
        }

        self.txs.write(messages);

        // Read rx fifos and put on tide fifos
        let messages = self.rxs.read();

        for (fifo, message) in self.tide_fifos.iter_mut().zip(messages.into_iter()) {
            for message in message {
                fifo.fifo
                    .push_back(message)
                    .expect("tide fifo not large enough")
            }
        }

        // Read one message from front of tide fifos and if necessary, put on SIO fifo.
        for (id, fifo) in self.tide_fifos.iter_mut().enumerate() {
            let message = fifo.fifo.pop_front();

            if let Some(message) = message {
                match message {
                    // Do nothing with sync messages, they're only in the fifo for sync
                    TideMessage::SyncMessage => (),
                    // Send comm message to core1
                    TideMessage::CommMessage {
                        neighbor: _,
                        data: _,
                    } => {
                        self.sio_fifo.write(message.serialize());
                    }
                }
            } else {
                log::warn!("FIFO #{} is empty.", id);
                // No other node is connected on this channel. No special action necessary.
            }
        }

        let buffer_levels: Vec<usize, N> = self.tide_fifos.iter().map(|f| f.fifo.len()).collect();

        self.frequency_controller.run(&buffer_levels);
    }
}

// TODO: not generic over N.., complicated by the different types.
pub struct Txs {
    pub tx0: Tx<(PIO1, SM0)>,
    pub tx1: Tx<(PIO1, SM1)>,
    pub tx2: Tx<(PIO1, SM2)>,
    pub tx3: Tx<(PIO1, SM3)>,
}

impl Txs {
    pub fn write(&mut self, messages: [TideMessage; 4]) {
        self.tx0.write(messages[0].serialize());
        self.tx1.write(messages[1].serialize());
        self.tx2.write(messages[2].serialize());
        self.tx3.write(messages[3].serialize());
    }
}

pub struct Rxs {
    pub rx0: Rx<(PIO0, SM0)>,
    pub rx1: Rx<(PIO0, SM1)>,
    pub rx2: Rx<(PIO0, SM2)>,
    pub rx3: Rx<(PIO0, SM3)>,
}

impl Rxs {
    /// Read at most 4 values from each RX fifo.
    /// The FIFOs hold 4 values, and if a neighbor is driving them faster than this node is running,
    /// it's possible for there to be more than one value present. So read exactly four times every
    /// time the control algo runs to keep up with clocks up to 4x this node's frequency.
    /// Also adjusts the message such that the neighbor field shows what neighbor it came from.
    pub fn read(&mut self) -> [Vec<TideMessage, 4>; 4] {
        macro_rules! read {
            ($rx:ident, $fifo_id:expr) => {
                (0..3)
                    .filter_map(|_| {
                        self.$rx.read().map(|w| {
                            let mut message = TideMessage::deserialize(w);

                            if let TideMessage::CommMessage { neighbor: _, data } = message {
                                message = TideMessage::CommMessage {
                                    neighbor: $fifo_id,
                                    data,
                                }
                            }

                            message
                        })
                    })
                    .collect::<Vec<_, 4>>()
            };
        }

        [read!(rx0, 0), read!(rx1, 1), read!(rx2, 2), read!(rx3, 3)]
    }
}

/// Generic over buffer size
pub struct TideFifo<const B: usize> {
    fifo: Deque<TideMessage, B>,
}

impl<const B: usize> TideFifo<B> {
    pub fn new() -> Self {
        let mut fifo = Deque::new();
        for _ in 0..(B / 2) {
            fifo.push_back(TideMessage::SyncMessage).unwrap()
        }
        Self { fifo }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TideMessage {
    /// Constant message used for sync purposes when no user message is ready
    SyncMessage,
    /// Communication message for user code. 1 bit is dedicated to signaling comm, 3 bits for the neigbor, the remaining 28 are for user data.
    CommMessage { neighbor: u8, data: u32 },
}

impl TideMessage {
    pub fn serialize(self) -> u32 {
        match self {
            TideMessage::SyncMessage => 0b0001,
            TideMessage::CommMessage { neighbor, data } => {
                let data = data & 0x0fff_ffff;
                let neighbor = neighbor & 0b111;

                (neighbor << 1) as u32 | (data << 4)
            }
        }
    }

    pub fn deserialize(raw: u32) -> Self {
        match raw {
            0b0001 => TideMessage::SyncMessage,
            raw => {
                let data = raw >> 4 & 0x0fff_ffff;
                let neighbor = (raw >> 1 & 0b111) as u8;
                TideMessage::CommMessage { neighbor, data }
            }
        }
    }
}
