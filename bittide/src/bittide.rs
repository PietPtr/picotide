use controllers::controller::FrequencyController;
use heapless::{Deque, Vec};

// TODO: debugging, starts with keeping track of things:
// - buffer levels for the last n iterations
// - ratio of sync messages vs data messages for last N iterations
// - buffer levels every m iterations for the last n * m iterations
// TODO: define ways to do a debug dump
// - SWD: possible, but interrupts bittide control flow due to large prints
// - over bittide network: serialize the debug dump in a stream of messages
//      which are sent instead of sync messages. These should be routed to
//      a controller / debugger, which can decode the messages and format
//      the dump. To do routing we need:
//      - topology discovery or definition
//      - routing of messages

/// Generic over the frequency controller F, and the buffer size B.
pub struct BittideChannelControl<F, const B: usize, L, const DEGREE: usize, FIFO> {
    frequency_controller: F,
    links: L,
    link_mask: [bool; DEGREE],
    sio_fifo: FIFO,
    tide_fifos: [BittideFifo<B>; DEGREE],
    debug_info: BittideChannelControlDebugInfo,
}

#[derive(Debug, Default)]
pub struct BittideChannelControlDebugInfo {
    pub buffer_levels: [u32; 4],
    pub rx_sync_message_counter: u32,
    pub rx_comm_message_counter: u32,
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
        link_mask: [bool; DEGREE],
        sio_fifo: FIFO,
        tide_fifos: [BittideFifo<B>; DEGREE],
    ) -> Self {
        Self {
            frequency_controller,
            links,
            link_mask,
            sio_fifo,
            tide_fifos,
            debug_info: BittideChannelControlDebugInfo::default(),
        }
    }

    /// All the logic to execute on a scheduled basis.
    /// This function must be called _exactly_ every `CLOCKS_PER_SYNC_WORD` system clock cycles.
    /// All clocks should be set up such that the execution of this function takes fewer clocks than that
    /// for its worst case execution path otherwise it cannot finish.
    pub fn interrupt(&mut self) -> Result<(), BittideChannelControlError> {
        // TODO: set error in debug info
        self.interrupt_internal()
    }

    fn interrupt_internal(&mut self) -> Result<(), BittideChannelControlError> {
        // Read user data from SIO FIFO
        let user_word = self.sio_fifo.read();

        // Send words on channel
        let mut messages = [BittideMessage::SyncMessage; DEGREE];

        if let Some(message) = user_word.map(BittideMessage::deserialize) {
            match message {
                BittideMessage::SyncMessage => {
                    return Err(BittideChannelControlError::SyncMessageFromUserCode)
                }
                BittideMessage::CommMessage { neighbor, data: _ } => {
                    let neighbor: usize = neighbor as usize;
                    if (neighbor) < 4 {
                        messages[neighbor] = message;
                    } else {
                        return Err(BittideChannelControlError::InvalidNeigbor);
                    }
                }
            }
        }

        self.links.write(messages);

        // Read rx fifos and put on tide fifos
        let messages = self.links.read();

        for (&enabled, (fifo, message)) in self
            .link_mask
            .iter()
            .zip(self.tide_fifos.iter_mut().zip(messages.into_iter()))
        {
            if !enabled {
                continue;
            }

            for message in message {
                match message {
                    BittideMessage::SyncMessage => self.debug_info.rx_sync_message_counter += 1,
                    BittideMessage::CommMessage {
                        neighbor: _,
                        data: _,
                    } => self.debug_info.rx_comm_message_counter += 1,
                }
                fifo.fifo
                    .push_back(message)
                    .map_err(|_| BittideChannelControlError::TideFifoFull)?;
                // TODO: write good error dump here with trace of last N fifo fill levels
            }
        }

        // Read one message from front of tide fifos and if necessary, put on SIO fifo.
        for (&enabled, (id, fifo)) in self
            .link_mask
            .iter()
            .zip(self.tide_fifos.iter_mut().enumerate())
        {
            if !enabled {
                continue;
            }

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
            self.tide_fifos.iter().map(|f| f.buffer_levels()).collect();

        self.debug_info
            .buffer_levels
            .iter_mut()
            .enumerate()
            .for_each(|(index, level)| {
                *level = self
                    .tide_fifos
                    .get(index)
                    .map(|fifo| fifo.buffer_levels())
                    .unwrap_or_default() as u32
            });

        let current_degree = self.links.active_fifos().iter().filter(|&&b| b).count();

        self.frequency_controller.set_degree(current_degree);
        self.frequency_controller
            .run(&buffer_levels)
            .map_err(|_| BittideChannelControlError::FrequenceControllerError)?;

        Ok(())
    }

    pub fn debug(&self) -> &BittideChannelControlDebugInfo {
        &self.debug_info
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

    pub fn buffer_levels(&self) -> usize {
        self.fifo.len()
    }
}

impl<const B: usize> Default for BittideFifo<B> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, defmt::Format)]
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

#[derive(Debug, defmt::Format)]
pub enum BittideChannelControlError {
    DecodeError,
    SyncMessageFromUserCode,
    InvalidNeigbor,
    TideFifoFull,
    FrequenceControllerError,
}

impl BittideChannelControlError {
    pub fn encode(result: Result<(), Self>) -> u32 {
        match result {
            Ok(()) => 0,
            Err(Self::DecodeError) => 1,
            Err(Self::SyncMessageFromUserCode) => 2,
            Err(Self::InvalidNeigbor) => 3,
            Err(Self::TideFifoFull) => 4,
            Err(Self::FrequenceControllerError) => 5,
        }
    }

    pub fn decode(value: u32) -> Result<(), Self> {
        match value {
            0 => Ok(()),
            2 => Err(Self::SyncMessageFromUserCode),
            3 => Err(Self::InvalidNeigbor),
            4 => Err(Self::TideFifoFull),
            5 => Err(Self::FrequenceControllerError),
            _ => Err(Self::DecodeError),
        }
    }
}
