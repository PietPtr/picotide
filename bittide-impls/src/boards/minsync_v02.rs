use bittide::bittide::{BittideChannelControl, BittideFifo};
use controllers::si5351::Si5351Controller;
use minsync::{
    hal::{
        gpio::{bank0::*, FunctionI2c, FunctionPio0, FunctionPio1, Pin, PullUp},
        pio::PIOExt,
        sio::SioFifo,
        I2C,
    },
    pac::{I2C1, PIO0, PIO1, RESETS},
};
use pitopi::{LinkConfig, Pitopi};
use rp_pico::hal::gpio::{bank0::Gpio0, DefaultTypeState};

use crate::chips::{self, rp2040::Rp2040Links};

pub const BUFFER_SIZE: usize = 64; // TODO: buffer size at compile time is inconvenient for fast iteration, what else can we use?

pub type Control = BittideChannelControl<
    Si5351Controller<si5351::Si5351Device<minsync::clocks::SiI2C>>,
    BUFFER_SIZE,
    Rp2040Links,
    4,
    crate::chips::rp2040::SioFifo,
>;

pub struct MinsyncPins {
    pub link: MinsyncLinkPins,
    pub rest: MinsyncRestPins,
}

impl From<minsync::Pins> for MinsyncPins {
    fn from(pins: minsync::Pins) -> Self {
        MinsyncPins {
            link: MinsyncLinkPins {
                north_0: pins.north_0,
                north_1: pins.north_1,
                north_2: pins.north_2,
                north_3: pins.north_3,
                north_4: pins.north_4,
                north_5: pins.north_5,
                east_6: pins.east_6,
                east_7: pins.east_7,
                east_8: pins.east_8,
                east_9: pins.east_9,
                east_10: pins.east_10,
                east_11: pins.east_11,
                south_16: pins.south_16,
                south_17: pins.south_17,
                south_18: pins.south_18,
                south_19: pins.south_19,
                south_21: pins.south_21,
                south_22: pins.south_22,
                west_23: pins.west_23,
                west_24: pins.west_24,
                west_26: pins.west_26,
                west_27: pins.west_27,
                west_28: pins.west_28,
                west_29: pins.west_29,
            },
            rest: MinsyncRestPins {
                oled_sda: pins.oled_sda,
                oled_scl: pins.oled_scl,
                si_sda: pins.si_sda,
                si_scl: pins.si_scl,
                led_or_si_clk1: pins.led_or_si_clk1,
                gpout3: pins.gpout3,
            },
        }
    }
}

pub struct MinsyncRestPins {
    pub oled_sda:
        Pin<Gpio12, <Gpio12 as DefaultTypeState>::Function, <Gpio12 as DefaultTypeState>::PullType>,
    pub oled_scl:
        Pin<Gpio13, <Gpio13 as DefaultTypeState>::Function, <Gpio13 as DefaultTypeState>::PullType>,
    pub si_sda:
        Pin<Gpio14, <Gpio14 as DefaultTypeState>::Function, <Gpio14 as DefaultTypeState>::PullType>,
    pub si_scl:
        Pin<Gpio15, <Gpio15 as DefaultTypeState>::Function, <Gpio15 as DefaultTypeState>::PullType>,
    pub led_or_si_clk1:
        Pin<Gpio20, <Gpio20 as DefaultTypeState>::Function, <Gpio20 as DefaultTypeState>::PullType>,
    pub gpout3:
        Pin<Gpio25, <Gpio25 as DefaultTypeState>::Function, <Gpio25 as DefaultTypeState>::PullType>,
}

pub struct MinsyncLinkPins {
    pub north_0:
        Pin<Gpio0, <Gpio0 as DefaultTypeState>::Function, <Gpio0 as DefaultTypeState>::PullType>,
    pub north_1:
        Pin<Gpio1, <Gpio1 as DefaultTypeState>::Function, <Gpio1 as DefaultTypeState>::PullType>,
    pub north_2:
        Pin<Gpio2, <Gpio2 as DefaultTypeState>::Function, <Gpio2 as DefaultTypeState>::PullType>,
    pub north_3:
        Pin<Gpio3, <Gpio3 as DefaultTypeState>::Function, <Gpio3 as DefaultTypeState>::PullType>,
    pub north_4:
        Pin<Gpio4, <Gpio4 as DefaultTypeState>::Function, <Gpio4 as DefaultTypeState>::PullType>,
    pub north_5:
        Pin<Gpio5, <Gpio5 as DefaultTypeState>::Function, <Gpio5 as DefaultTypeState>::PullType>,
    pub east_6:
        Pin<Gpio6, <Gpio6 as DefaultTypeState>::Function, <Gpio6 as DefaultTypeState>::PullType>,
    pub east_7:
        Pin<Gpio7, <Gpio7 as DefaultTypeState>::Function, <Gpio7 as DefaultTypeState>::PullType>,
    pub east_8:
        Pin<Gpio8, <Gpio8 as DefaultTypeState>::Function, <Gpio8 as DefaultTypeState>::PullType>,
    pub east_9:
        Pin<Gpio9, <Gpio9 as DefaultTypeState>::Function, <Gpio9 as DefaultTypeState>::PullType>,
    pub east_10:
        Pin<Gpio10, <Gpio10 as DefaultTypeState>::Function, <Gpio10 as DefaultTypeState>::PullType>,
    pub east_11:
        Pin<Gpio11, <Gpio11 as DefaultTypeState>::Function, <Gpio11 as DefaultTypeState>::PullType>,
    pub south_16:
        Pin<Gpio16, <Gpio16 as DefaultTypeState>::Function, <Gpio16 as DefaultTypeState>::PullType>,
    pub south_17:
        Pin<Gpio17, <Gpio17 as DefaultTypeState>::Function, <Gpio17 as DefaultTypeState>::PullType>,
    pub south_18:
        Pin<Gpio18, <Gpio18 as DefaultTypeState>::Function, <Gpio18 as DefaultTypeState>::PullType>,
    pub south_19:
        Pin<Gpio19, <Gpio19 as DefaultTypeState>::Function, <Gpio19 as DefaultTypeState>::PullType>,
    pub south_21:
        Pin<Gpio21, <Gpio21 as DefaultTypeState>::Function, <Gpio21 as DefaultTypeState>::PullType>,
    pub south_22:
        Pin<Gpio22, <Gpio22 as DefaultTypeState>::Function, <Gpio22 as DefaultTypeState>::PullType>,
    pub west_23:
        Pin<Gpio23, <Gpio23 as DefaultTypeState>::Function, <Gpio23 as DefaultTypeState>::PullType>,
    pub west_24:
        Pin<Gpio24, <Gpio24 as DefaultTypeState>::Function, <Gpio24 as DefaultTypeState>::PullType>,
    pub west_26:
        Pin<Gpio26, <Gpio26 as DefaultTypeState>::Function, <Gpio26 as DefaultTypeState>::PullType>,
    pub west_27:
        Pin<Gpio27, <Gpio27 as DefaultTypeState>::Function, <Gpio27 as DefaultTypeState>::PullType>,
    pub west_28:
        Pin<Gpio28, <Gpio28 as DefaultTypeState>::Function, <Gpio28 as DefaultTypeState>::PullType>,
    pub west_29:
        Pin<Gpio29, <Gpio29 as DefaultTypeState>::Function, <Gpio29 as DefaultTypeState>::PullType>,
}

pub struct MinsyncV02 {}
impl MinsyncV02 {
    pub fn setup(
        link_mask: [bool; 4],
        frequency_controller: Si5351Controller<si5351::Si5351Device<minsync::clocks::SiI2C>>,
        pins: MinsyncLinkPins,
        pio0: PIO0,
        pio1: PIO1,
        resets: &mut RESETS,
        sio_fifo: SioFifo,
    ) -> Control {
        let (rx_pio, rx_sm0, rx_sm1, rx_sm2, rx_sm3) = pio0.split(resets);
        let (tx_pio, tx_sm0, tx_sm1, tx_sm2, tx_sm3) = pio1.split(resets);

        let rx0_data = pins.north_3.into_function::<FunctionPio0>().into_dyn_pin();
        let rx0_word = pins.north_4.into_function::<FunctionPio0>().into_dyn_pin();
        let rx0_clk = pins.north_5.into_function::<FunctionPio0>().into_dyn_pin();

        let rx1_data = pins.east_9.into_function::<FunctionPio0>().into_dyn_pin();
        let rx1_word = pins.east_10.into_function::<FunctionPio0>().into_dyn_pin();
        let rx1_clk = pins.east_11.into_function::<FunctionPio0>().into_dyn_pin();

        let rx2_data = pins.south_19.into_function::<FunctionPio0>().into_dyn_pin();
        let rx2_word = pins.south_21.into_function::<FunctionPio0>().into_dyn_pin();
        let rx2_clk = pins.south_22.into_function::<FunctionPio0>().into_dyn_pin();

        let rx3_data = pins.west_27.into_function::<FunctionPio0>().into_dyn_pin();
        let rx3_word = pins.west_28.into_function::<FunctionPio0>().into_dyn_pin();
        let rx3_clk = pins.west_29.into_function::<FunctionPio0>().into_dyn_pin();

        let tx0_data = pins.north_2.into_function::<FunctionPio1>().into_dyn_pin();
        let tx0_word = pins.north_1.into_function::<FunctionPio1>().into_dyn_pin();
        let tx0_clk = pins.north_0.into_function::<FunctionPio1>().into_dyn_pin();

        let tx1_data = pins.east_8.into_function::<FunctionPio1>().into_dyn_pin();
        let tx1_word = pins.east_7.into_function::<FunctionPio1>().into_dyn_pin();
        let tx1_clk = pins.east_6.into_function::<FunctionPio1>().into_dyn_pin();

        let tx2_data = pins.south_18.into_function::<FunctionPio1>().into_dyn_pin();
        let tx2_word = pins.south_17.into_function::<FunctionPio1>().into_dyn_pin();
        let tx2_clk = pins.south_16.into_function::<FunctionPio1>().into_dyn_pin();

        let tx3_data = pins.west_26.into_function::<FunctionPio1>().into_dyn_pin();
        let tx3_word = pins.west_24.into_function::<FunctionPio1>().into_dyn_pin();
        let tx3_clk = pins.west_23.into_function::<FunctionPio1>().into_dyn_pin();

        let mut pitopi = Pitopi::new(rx_pio, tx_pio);

        pitopi.install_programs();

        let (_, rx0, _, tx0) = pitopi
            .setup_link(
                pitopi::DEFAULT_LINK_CONFIG,
                rx_sm0,
                rx0_data,
                rx0_clk,
                rx0_word,
                tx_sm0,
                tx0_data,
                tx0_clk,
                tx0_word,
            )
            .unwrap();

        let (_, rx1, _, tx1) = pitopi
            .setup_link(
                pitopi::DEFAULT_LINK_CONFIG,
                rx_sm1,
                rx1_data,
                rx1_clk,
                rx1_word,
                tx_sm1,
                tx1_data,
                tx1_clk,
                tx1_word,
            )
            .unwrap();

        let south_link_config = LinkConfig {
            rx_program: pitopi::RxProgram::P023,
            tx_program: pitopi::TxProgram::SidesetWC,
        };

        let (_, rx2, _, tx2) = pitopi
            .setup_link(
                south_link_config,
                rx_sm2,
                rx2_data,
                rx2_clk,
                rx2_word,
                tx_sm2,
                tx2_data,
                tx2_clk,
                tx2_word,
            )
            .unwrap();

        let (_, rx3, _, tx3) = pitopi
            .setup_link(
                pitopi::DEFAULT_LINK_CONFIG,
                rx_sm3,
                rx3_data,
                rx3_clk,
                rx3_word,
                tx_sm3,
                tx3_data,
                tx3_clk,
                tx3_word,
            )
            .unwrap();

        let tide_fifos = [
            BittideFifo::new(),
            BittideFifo::new(),
            BittideFifo::new(),
            BittideFifo::new(),
        ];

        Control::new(
            frequency_controller,
            Rp2040Links::new(rx0, rx1, rx2, rx3, tx0, tx1, tx2, tx3),
            link_mask,
            chips::rp2040::SioFifo(sio_fifo),
            tide_fifos,
        )
    }
}
