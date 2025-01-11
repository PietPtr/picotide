use controllers::controller::FrequencyController;
use heapless::{Deque, Vec};

/// Generic over the frequency controller F, and the buffer size B.
pub struct BittideChannelControl<F, const B: usize, L, const DEGREE: usize, FIFO> {
    frequency_controller: F,
    links: L,
    sio_fifo: FIFO,
    tide_fifos: [BittideFifo<B>; DEGREE],
}

impl<F, const B: usize, L, const DEGREE: usize, FIFO> BittideChannelControl<F, B, L, DEGREE, FIFO>
where
    F: FrequencyController<B>,
    L: Links<DEGREE>,
    FIFO: Fifo,
{
    pub fn new(
        frequency_controller: F,
        links: L,
        sio_fifo: FIFO,
        tide_fifos: [BittideFifo<B>; DEGREE],
    ) -> Self {
        Self {
            frequency_controller,
            links,
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
        let mut messages = [BittideMessage::SyncMessage; DEGREE];

        if let Some(message) = user_word.map(BittideMessage::deserialize) {
            match message {
                BittideMessage::SyncMessage => panic!("unexpected"),
                BittideMessage::CommMessage { neighbor, data: _ } => {
                    let neighbor: usize = neighbor as usize;
                    if (neighbor) < 4 {
                        messages[neighbor] = message;
                    } else {
                        panic!("Invalid neigbor selected");
                    }
                }
            }
        }

        self.links.write(messages);

        // Read rx fifos and put on tide fifos
        let messages = self.links.read();

        for (fifo, message) in self.tide_fifos.iter_mut().zip(messages.into_iter()) {
            for message in message {
                fifo.fifo
                    .push_back(message)
                    .expect("tide fifo not large enough")
                // TODO: write good error dump here with trace of last N fifo fill levels
            }
        }

        // Read one message from front of tide fifos and if necessary, put on SIO fifo.
        for (id, fifo) in self.tide_fifos.iter_mut().enumerate() {
            let message = fifo.fifo.pop_front();

            if let Some(message) = message {
                match message {
                    // Do nothing with sync messages, they're only in the fifo for sync
                    BittideMessage::SyncMessage => (),
                    // Send comm message to core1
                    BittideMessage::CommMessage {
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

        let buffer_levels: Vec<usize, DEGREE> =
            self.tide_fifos.iter().map(|f| f.fifo.len()).collect();

        self.frequency_controller
            .set_degree(self.links.active_fifos().iter().filter(|&&b| b).count());
        self.frequency_controller.run(&buffer_levels);
    }
}

/// Encapsulates all hardware resources for all possible bittide links for a device.
/// Methods should implement a read and write on every link available.
pub trait Links<const DEGREE: usize> {
    fn write(&mut self, messages: [BittideMessage; DEGREE]);
    fn read(&mut self) -> [Vec<BittideMessage, 4>; DEGREE];
    fn active_fifos(&self) -> [bool; DEGREE];
}

/// A FIFO-like object to transfer data words to and from the process.
pub trait Fifo {
    fn read(&mut self) -> Option<u32>;
    fn write(&mut self, data: u32);
}

/// Generic over buffer size
pub struct BittideFifo<const B: usize> {
    fifo: Deque<BittideMessage, B>,
}

impl<const B: usize> BittideFifo<B> {
    pub fn new() -> Self {
        let mut fifo = Deque::new();
        for _ in 0..(B / 2) {
            fifo.push_back(BittideMessage::SyncMessage).unwrap()
        }
        Self { fifo }
    }
}

impl<const B: usize> Default for BittideFifo<B> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BittideMessage {
    /// Constant message used for sync purposes when no user message is ready
    SyncMessage,
    /// Communication message for user code. 1 bit is dedicated to signaling comm, 3 bits for the neigbor, the remaining 28 are for user data.
    CommMessage { neighbor: u8, data: u32 },
}

impl BittideMessage {
    pub fn serialize(self) -> u32 {
        match self {
            BittideMessage::SyncMessage => 0b0001,
            BittideMessage::CommMessage { neighbor, data } => {
                let data = data & 0x0fff_ffff;
                let neighbor = neighbor & 0b111;

                (neighbor << 1) as u32 | (data << 4)
            }
        }
    }

    pub fn deserialize(raw: u32) -> Self {
        match raw {
            0b0001 => BittideMessage::SyncMessage,
            raw => {
                let data = raw >> 4 & 0x0fff_ffff;
                let neighbor = (raw >> 1 & 0b111) as u8;
                BittideMessage::CommMessage { neighbor, data }
            }
        }
    }
}
