use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    pin: u8,
}

fn main() {
    let args = Args::parse();
    scripts::usb::usb(args.pin).expect("Set SWDIO pin failed.");
}
