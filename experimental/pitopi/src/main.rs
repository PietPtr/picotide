#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::{cell::RefCell, u32};

use controllers::{controller::FrequencyController, fbdiv::FbdivController};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::exception;
use critical_section::Mutex;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use fugit::HertzU32;
use heapless::{Deque, Vec};
use panic_probe as _;
use pio_proc::pio_file;
use rp_pico::{
    entry,
    hal::{
        clocks::{Clock, ClockSource, ClocksManager},
        gpio::{self, FunctionPio0, FunctionPio1},
        pio::{PIOBuilder, PIOExt, PinDir, Rx, Tx, SM0, SM1, SM2, SM3},
        pll::{setup_pll_blocking, PLLConfig},
        sio::{Sio, SioFifo},
        xosc::setup_xosc_blocking,
    },
    pac::{self, PIO0, PIO1},
};

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1600),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
pub const CLOCKS_PER_SYNC_WORD: u32 = 1024;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let sio = Sio::new(pac.SIO);

    let mut clocks = ClocksManager::new(pac.CLOCKS);

    let xosc = setup_xosc_blocking(pac.XOSC, EXTERNAL_XTAL_FREQ_HZ).unwrap();

    let locked_pll_sys = setup_pll_blocking(
        pac.PLL_SYS,
        xosc.operating_frequency(),
        SYS_PLL_CONFIG_100MHZ,
        &mut clocks,
        &mut pac.RESETS,
    )
    .unwrap();

    clocks
        .system_clock
        .configure_clock(&locked_pll_sys, locked_pll_sys.get_freq())
        .unwrap();

    info!(
        "Configured system clock at frequency: {:?}MHz",
        locked_pll_sys.get_freq().to_Hz() as f32 / 1e6
    );

    let pll_sys = locked_pll_sys.free();

    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let (mut tx_pio, tx_sm0, tx_sm1, tx_sm2, tx_sm3) = pac.PIO1.split(&mut pac.RESETS);
    let (mut rx_pio, rx_sm0, rx_sm1, rx_sm2, rx_sm3) = pac.PIO0.split(&mut pac.RESETS);

    // Set up 4 TX channels
    let tx0_data = pins.gpio0.into_function::<FunctionPio1>();
    let tx0_clk = pins.gpio1.into_function::<FunctionPio1>();
    let tx0_word = pins.gpio2.into_function::<FunctionPio1>();

    let tx1_data = pins.gpio6.into_function::<FunctionPio1>();
    let tx1_clk = pins.gpio7.into_function::<FunctionPio1>();
    let tx1_word = pins.gpio8.into_function::<FunctionPio1>();

    let tx2_data = pins.gpio12.into_function::<FunctionPio1>();
    let tx2_clk: gpio::Pin<gpio::bank0::Gpio13, FunctionPio1, gpio::PullDown> =
        pins.gpio13.into_function::<FunctionPio1>();
    let tx2_word = pins.gpio14.into_function::<FunctionPio1>();

    let tx3_data = pins.gpio18.into_function::<FunctionPio1>();
    let tx3_clk = pins.gpio19.into_function::<FunctionPio1>();
    let tx3_word = pins.gpio20.into_function::<FunctionPio1>();

    let pitopi_tx_program = pio_file!("src/programs.pio", select_program("pitopi_tx")).program;
    let tx_program = tx_pio.install(&pitopi_tx_program).unwrap();

    let (mut tx_sm0, _rx0, tx0) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx0_data.id().num, 1)
        .side_set_pin_base(tx0_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm0);

    tx_sm0.set_pindirs([
        (tx0_data.id().num, PinDir::Output),
        (tx0_clk.id().num, PinDir::Output),
        (tx0_word.id().num, PinDir::Output),
    ]);

    tx_sm0.start();

    let (mut tx_sm1, _rx1, tx1) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx1_data.id().num, 1)
        .side_set_pin_base(tx1_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm1);

    tx_sm1.set_pindirs([
        (tx1_data.id().num, PinDir::Output),
        (tx1_clk.id().num, PinDir::Output),
        (tx1_word.id().num, PinDir::Output),
    ]);

    tx_sm1.start();

    let (mut tx_sm2, _rx2, tx2) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx2_data.id().num, 1)
        .side_set_pin_base(tx2_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm2);

    tx_sm2.set_pindirs([
        (tx2_data.id().num, PinDir::Output),
        (tx2_clk.id().num, PinDir::Output),
        (tx2_word.id().num, PinDir::Output),
    ]);

    tx_sm2.start();

    let (mut tx_sm3, _rx3, tx3) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx3_data.id().num, 1)
        .side_set_pin_base(tx3_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm3);

    tx_sm3.set_pindirs([
        (tx3_data.id().num, PinDir::Output),
        (tx3_clk.id().num, PinDir::Output),
        (tx3_word.id().num, PinDir::Output),
    ]);

    tx_sm3.start();

    let rx0_data = pins.gpio3.into_function::<FunctionPio0>();
    let rx0_clk = pins.gpio4.into_function::<FunctionPio0>();
    let rx0_word = pins.gpio5.into_function::<FunctionPio0>();

    let rx1_data = pins.gpio9.into_function::<FunctionPio0>();
    let rx1_clk = pins.gpio10.into_function::<FunctionPio0>();
    let rx1_word = pins.gpio11.into_function::<FunctionPio0>();

    let rx2_data = pins.gpio15.into_function::<FunctionPio0>();
    let rx2_clk = pins.gpio16.into_function::<FunctionPio0>();
    let rx2_word = pins.gpio17.into_function::<FunctionPio0>();

    let rx3_data = pins.gpio21.into_function::<FunctionPio0>();
    let rx3_clk = pins.gpio22.into_function::<FunctionPio0>();
    let rx3_word = pins.gpio23.into_function::<FunctionPio0>();

    let pitopi_rx_program = pio_file!("src/programs.pio", select_program("pitopi_rx")).program;
    let rx_program = rx_pio.install(&pitopi_rx_program).unwrap();

    let (mut rx_sm0, rx0, _tx0) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx0_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm0);

    rx_sm0.set_pindirs([
        (rx0_data.id().num, PinDir::Input),
        (rx0_clk.id().num, PinDir::Input),
        (rx0_word.id().num, PinDir::Input),
    ]);

    rx_sm0.start();

    let (mut rx_sm1, rx1, _tx1) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx1_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm1);

    rx_sm1.set_pindirs([
        (rx1_data.id().num, PinDir::Input),
        (rx1_clk.id().num, PinDir::Input),
        (rx1_word.id().num, PinDir::Input),
    ]);

    rx_sm1.start();

    let (mut rx_sm2, rx2, _tx2) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx2_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm2);

    rx_sm2.set_pindirs([
        (rx2_data.id().num, PinDir::Input),
        (rx2_clk.id().num, PinDir::Input),
        (rx2_word.id().num, PinDir::Input),
    ]);

    rx_sm2.start();

    let (mut rx_sm3, rx3, _tx3) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx3_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm3);

    rx_sm3.set_pindirs([
        (rx3_data.id().num, PinDir::Input),
        (rx3_clk.id().num, PinDir::Input),
        (rx3_word.id().num, PinDir::Input),
    ]);

    rx_sm3.start();

    info!("Start.");

    let sio_fifo = sio.fifo;

    let tide_fifos = [
        TideFifo::new(),
        TideFifo::new(),
        TideFifo::new(),
        TideFifo::new(),
    ];

    critical_section::with(|cs| {
        GLOBAL_CONTROL.borrow(cs).replace(Some(Control::new(
            FbdivController::new(pll_sys, 1),
            Rxs { rx0, rx1, rx2, rx3 },
            Txs { tx0, tx1, tx2, tx3 },
            sio_fifo,
            tide_fifos,
        )))
    });

    let mut systick = core.SYST;
    systick.set_reload(CLOCKS_PER_SYNC_WORD - 1);
    systick.clear_current();
    systick.enable_counter();
    systick.set_clock_source(SystClkSource::Core);
    // systick.enable_interrupt();

    info!(
        "\nclksource={} ({})\nenabled={}\ntickint={}\nrvr={:#x}\nsyst_calib: noref={} skew={} tenms={:x}",
        if pac.PPB.syst_csr.read().clksource().bit() {
            "processor"
        } else {
            "refclock"
        },
        pac.PPB.syst_csr.read().clksource().bit(),
        pac.PPB.syst_csr.read().enable().bit_is_set(),
        pac.PPB.syst_csr.read().tickint().bit(),
        pac.PPB.syst_rvr.read().bits(),
        pac.PPB.syst_calib.read().noref().bit(),
        pac.PPB.syst_calib.read().skew().bit(),
        pac.PPB.syst_calib.read().tenms().bits(),
    );

    loop {}
}

type Control = TideChannelControl<FbdivController, 4, 16>;

const GLOBAL_CONTROL: Mutex<RefCell<Option<Control>>> = Mutex::new(RefCell::new(None));

#[exception]
fn SysTick() {
    static mut CONTROL: Option<Control> = None;

    if CONTROL.is_none() {
        critical_section::with(|cs| {
            let _ = CONTROL.insert(GLOBAL_CONTROL.borrow(cs).take().unwrap());
        });
    }

    if let Some(control) = CONTROL {
        control.interrupt();
    }
}

/// Generic over the frequency controller F, the amount of neighbors N
/// and the buffer size B.
/// TODO: Generic N does not fully work as Rxs/Txs are hardcoded to size 4.
/// TODO: make this a library
struct TideChannelControl<F, const N: usize, const B: usize> {
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
    /// This function must be called _exactly_ every `CLOCKS_PER_SYNC_WORD` cycles.
    /// All clocks should be set up such that the execution of this function takes fewer clocks than that
    /// for its worst case execution path otherwise it cannot finish.
    pub fn interrupt(&mut self) {
        // Read user data from SIO FIFO
        let user_word = self.sio_fifo.read();

        // Send words on channel
        let mut messages: [TideMessage; 4] = [
            TideMessage::SyncMessage,
            TideMessage::SyncMessage,
            TideMessage::SyncMessage,
            TideMessage::SyncMessage,
        ];

        if let Some(message) = user_word.map(|w| TideMessage::deserialize(w)) {
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
        for fifo in self.tide_fifos.iter_mut() {
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
                panic!("Empty tide fifo!")
            }
        }

        let buffer_levels: Vec<usize, N> = self.tide_fifos.iter().map(|f| f.fifo.len()).collect();

        self.frequency_controller.run(&buffer_levels);
    }
}

// TODO: not generic over N.., complicated by the different types.
struct Txs {
    tx0: Tx<(PIO1, SM0)>,
    tx1: Tx<(PIO1, SM1)>,
    tx2: Tx<(PIO1, SM2)>,
    tx3: Tx<(PIO1, SM3)>,
}

impl Txs {
    pub fn write(&mut self, messages: [TideMessage; 4]) {
        self.tx0.write(messages[0].serialize());
        self.tx1.write(messages[1].serialize());
        self.tx2.write(messages[2].serialize());
        self.tx3.write(messages[3].serialize());
    }
}

struct Rxs {
    rx0: Rx<(PIO0, SM0)>,
    rx1: Rx<(PIO0, SM1)>,
    rx2: Rx<(PIO0, SM2)>,
    rx3: Rx<(PIO0, SM3)>,
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
struct TideFifo<const B: usize> {
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
enum TideMessage {
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
