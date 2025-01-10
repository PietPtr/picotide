use pio_proc::pio_file;
use rp_pico::{
    hal::{
        gpio::{DynPinId, Function, Pin, PullDown},
        pio::{
            InstalledProgram, PIOBuilder, PinDir, Running, Rx, StateMachine, StateMachineIndex, Tx,
            UninitStateMachine, PIO,
        },
    },
    pac::{PIO0, PIO1},
};

pub struct Pitopi {
    rx_pio: PIO<PIO0>,
    tx_pio: PIO<PIO1>,

    rx_program: Option<InstalledProgram<PIO0>>,
    tx_program: Option<InstalledProgram<PIO1>>,
}

type LinkStateMachines<RXSM, TXSM> = (
    StateMachine<(PIO0, RXSM), Running>,
    Rx<(PIO0, RXSM)>,
    StateMachine<(PIO1, TXSM), Running>,
    Tx<(PIO1, TXSM)>,
);

impl Pitopi {
    pub fn install_programs(&mut self) {
        let pitopi_tx_program = pio_file!("src/programs.pio", select_program("pitopi_tx")).program;
        self.tx_program = Some(self.tx_pio.install(&pitopi_tx_program).unwrap());

        let pitopi_rx_program = pio_file!("src/programs.pio", select_program("pitopi_rx")).program;
        self.rx_program = Some(self.rx_pio.install(&pitopi_rx_program).unwrap());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn setup_link<FPIO: Function, RXSM, TXSM>(
        &mut self,
        rx_sm: UninitStateMachine<(PIO0, RXSM)>,
        rx_data_pin: Pin<DynPinId, FPIO, PullDown>,
        rx_clk_pin: Pin<DynPinId, FPIO, PullDown>,
        rx_word_pin: Pin<DynPinId, FPIO, PullDown>,
        tx_sm: UninitStateMachine<(PIO1, TXSM)>,
        tx_data_pin: Pin<DynPinId, FPIO, PullDown>,
        tx_clk_pin: Pin<DynPinId, FPIO, PullDown>,
        tx_word_pin: Pin<DynPinId, FPIO, PullDown>,
    ) -> Result<LinkStateMachines<RXSM, TXSM>, PitopiError>
    where
        RXSM: StateMachineIndex,
        TXSM: StateMachineIndex,
    {
        let Some(rx_program) = self.rx_program.as_mut() else {
            return Err(PitopiError::RxProgramNotInstalled);
        };

        let (mut rx_sm, rx_fifo, _) =
            PIOBuilder::from_installed_program(unsafe { rx_program.share() })
                .in_pin_base(rx_data_pin.id().num)
                .clock_divisor_fixed_point(1, 0)
                .build(rx_sm);

        rx_sm.set_pindirs([
            (rx_data_pin.id().num, PinDir::Input),
            (rx_clk_pin.id().num, PinDir::Input),
            (rx_word_pin.id().num, PinDir::Input),
        ]);

        let rx_sm = rx_sm.start();

        let Some(tx_program) = self.tx_program.as_mut() else {
            return Err(PitopiError::TxProgramNotInstalled);
        };

        let (mut tx_sm, _, tx_fifo) =
            PIOBuilder::from_installed_program(unsafe { tx_program.share() })
                .out_pins(tx_data_pin.id().num, 1)
                .side_set_pin_base(tx_clk_pin.id().num)
                .clock_divisor_fixed_point(4, 0)
                .build(tx_sm);

        tx_sm.set_pindirs([
            (tx_data_pin.id().num, PinDir::Output),
            (tx_clk_pin.id().num, PinDir::Output),
            (tx_word_pin.id().num, PinDir::Output),
        ]);

        let tx_sm = tx_sm.start();

        Ok((rx_sm, rx_fifo, tx_sm, tx_fifo))
    }
}

pub enum PitopiError {
    TxProgramNotInstalled,
    RxProgramNotInstalled,
}
