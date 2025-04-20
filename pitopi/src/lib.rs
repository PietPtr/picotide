#![no_std]
use pio_proc::pio_file;
use rp_pico::{
    hal::{
        gpio::{DynPinId, FunctionPio0, FunctionPio1, Pin, PullDown},
        pio::{
            InstalledProgram, PIOBuilder, PinDir, Running, Rx, StateMachine, StateMachineIndex, Tx,
            UninitStateMachine, PIO,
        },
    },
    pac::{PIO0, PIO1},
};

pub const DEFAULT_LINK_CONFIG: LinkConfig = LinkConfig {
    rx_program: RxProgram::Consecutive,
    tx_program: TxProgram::SidesetWC,
};

pub struct Pitopi {
    rx_pio: PIO<PIO0>,
    tx_pio: PIO<PIO1>,

    rx_program: Option<InstalledProgram<PIO0>>,
    rx_program_023: Option<InstalledProgram<PIO0>>,
    tx_program: Option<InstalledProgram<PIO1>>,
}

type LinkStateMachines<RXSM, TXSM> = (
    StateMachine<(PIO0, RXSM), Running>,
    Rx<(PIO0, RXSM)>,
    StateMachine<(PIO1, TXSM), Running>,
    Tx<(PIO1, TXSM)>,
);

impl Pitopi {
    pub fn new(rx_pio: PIO<PIO0>, tx_pio: PIO<PIO1>) -> Self {
        Self {
            rx_pio,
            tx_pio,
            rx_program: None,
            rx_program_023: None,
            tx_program: None,
        }
    }

    pub fn install_programs(&mut self) {
        let pitopi_tx_program = pio_file!("src/programs.pio", select_program("pitopi_tx")).program;
        self.tx_program = Some(self.tx_pio.install(&pitopi_tx_program).unwrap());

        let pitopi_rx_program = pio_file!("src/programs.pio", select_program("pitopi_rx")).program;
        self.rx_program = Some(self.rx_pio.install(&pitopi_rx_program).unwrap());

        let pitopi_rx_program_023 =
            pio_file!("src/programs.pio", select_program("pitopi_rx_023")).program;
        self.rx_program_023 = Some(self.rx_pio.install(&pitopi_rx_program_023).unwrap());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn setup_link<RXSM, TXSM>(
        &mut self,
        link_config: LinkConfig,
        rx_sm: UninitStateMachine<(PIO0, RXSM)>,
        rx_data_pin: Pin<DynPinId, FunctionPio0, PullDown>,
        rx_clk_pin: Pin<DynPinId, FunctionPio0, PullDown>,
        rx_word_pin: Pin<DynPinId, FunctionPio0, PullDown>,
        tx_sm: UninitStateMachine<(PIO1, TXSM)>,
        tx_data_pin: Pin<DynPinId, FunctionPio1, PullDown>,
        tx_clk_pin: Pin<DynPinId, FunctionPio1, PullDown>,
        tx_word_pin: Pin<DynPinId, FunctionPio1, PullDown>,
    ) -> Result<LinkStateMachines<RXSM, TXSM>, PitopiError>
    where
        RXSM: StateMachineIndex,
        TXSM: StateMachineIndex,
    {
        // For the PIO programs to work, the RX pins must be strictly consecutive, in the order clock, word, data.
        // assert_eq!(rx_clk_pin.id().num + 1, rx_word_pin.id().num);
        // assert_eq!(rx_clk_pin.id().num + 2, rx_data_pin.id().num);
        // Also, the tx word and clk pin must be consecutive in that order (if the default TX program is used and not the mirrored)
        // assert_eq!(tx_word_pin.id().num + 1, tx_clk_pin.id().num);

        let rx_program = match link_config.rx_program {
            RxProgram::Consecutive => &mut self.rx_program,
            RxProgram::P023 => &mut self.rx_program_023,
        };

        let Some(rx_program) = rx_program.as_mut() else {
            return Err(PitopiError::RxProgramNotInstalled);
        };

        let rx_base_pin = rx_data_pin.id().num;

        let (mut rx_sm, rx_fifo, _) =
            PIOBuilder::from_installed_program(unsafe { rx_program.share() })
                .in_pin_base(rx_base_pin)
                .clock_divisor_fixed_point(1, 0)
                .build(rx_sm);

        rx_sm.set_pindirs([
            (rx_data_pin.id().num, PinDir::Input),
            (rx_word_pin.id().num, PinDir::Input),
            (rx_clk_pin.id().num, PinDir::Input),
        ]);

        match link_config.rx_program {
            RxProgram::Consecutive => {
                defmt::info!(
                    "cons | setting up link, computed pins:\ndata = gpio{}\nword = gpio{}\nclk  = gpio{}",
                    rx_base_pin,
                    rx_base_pin + 1,
                    rx_base_pin + 2
                );
            }
            RxProgram::P023 => {
                defmt::info!(
                    "p023 | setting up link, computed pins:\ndata = gpio{}\nword = gpio{}\nclk  = gpio{}",
                    rx_base_pin,
                    rx_base_pin + 2,
                    rx_base_pin + 3
                );
            }
        }

        let rx_sm = rx_sm.start();

        let Some(tx_program) = self.tx_program.as_mut() else {
            return Err(PitopiError::TxProgramNotInstalled);
        };

        // default
        let (mut tx_sm, _, tx_fifo) =
            PIOBuilder::from_installed_program(unsafe { tx_program.share() })
                .out_pins(tx_data_pin.id().num, 1)
                .side_set_pin_base(tx_clk_pin.id().num)
                .clock_divisor_fixed_point(64, 0)
                .build(tx_sm);

        tx_sm.set_pindirs([
            (tx_data_pin.id().num, PinDir::Output),
            (tx_clk_pin.id().num, PinDir::Output),
            (tx_word_pin.id().num, PinDir::Output),
        ]);

        // mirror
        // let (mut tx_sm, _, tx_fifo) =
        //     PIOBuilder::from_installed_program(unsafe { tx_program.share() })
        //         .out_pins(tx_data_pin.id().num, 1)
        //         .side_set_pin_base(tx_word_pin.id().num)
        //         .clock_divisor_fixed_point(64, 0)
        //         .build(tx_sm);

        // tx_sm.set_pindirs([
        //     (tx_data_pin.id().num, PinDir::Output),
        //     (tx_clk_pin.id().num, PinDir::Output),
        //     (tx_word_pin.id().num, PinDir::Output),
        // ]);

        let tx_sm = tx_sm.start();

        Ok((rx_sm, rx_fifo, tx_sm, tx_fifo))
    }

    pub fn free(self) -> (PIO<PIO0>, PIO<PIO1>) {
        (self.rx_pio, self.tx_pio)
    }
}

pub struct LinkConfig {
    pub rx_program: RxProgram,
    pub tx_program: TxProgram,
}

pub enum RxProgram {
    Consecutive,
    P023,
}

pub enum TxProgram {
    SidesetWC,
}

#[derive(Debug)]
pub enum PitopiError {
    TxProgramNotInstalled,
    RxProgramNotInstalled,
}
