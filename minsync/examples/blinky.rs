#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use cortex_m::asm;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_hal::PwmPin;
use fugit::HertzU32;
use panic_probe as _;
use rp2040_hal::Watchdog;

use minsync::{
    entry,
    hal::{
        self,
        clocks::{ClockSource, ClocksManager},
        pac,
        pll::PLLConfig,
        rosc::RingOscillator,
        Clock,
    },
};

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1200),
    refdiv: 1,
    post_div1: 5,
    post_div2: 2,
};

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();
    let sio = hal::Sio::new(pac.SIO);

    let watchdog = Watchdog::new(pac.WATCHDOG);
    watchdog.disable();

    let pins = minsync::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    minsync::clocks::minimal_clock_setup(pac.CLOCKS, pac.ROSC, pins.gpout3)
        .expect("Failed to do basic clock set up.");

    info!("Hello!");

    // Breathe the LED with the PWM.
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    let pwm = &mut pwm_slices.pwm2;
    pwm.set_ph_correct();
    pwm.enable();
    let channel = &mut pwm.channel_a;
    channel.set_duty(5000);
    channel.output_to(pins.led_or_si_clk1);

    let mut duty = 0i32;
    let mut going_up = true;

    loop {
        channel.set_duty(duty as u16);

        for _ in 0..50 {
            asm::nop();
        }

        duty += if going_up { 1 } else { -1 };

        if duty == u16::MAX as i32 {
            going_up = false;
        }

        if duty == 0 {
            going_up = true
        }
    }
}
