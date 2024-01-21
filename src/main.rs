#![cfg_attr(not(test), no_std)]
#![no_main]
#![feature(type_alias_impl_trait)]
#![forbid(unsafe_code)]
#![allow(rustdoc::bare_urls)]

use crate::{formatter::Formatter, progressbar::ProgressBar};
use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    config::Config,
    gpio::{Level, Output},
    i2c,
    peripherals::{I2C0, PIN_25},
    watchdog::Watchdog,
};
use embassy_time::Timer;
use embedded_graphics::{
    mono_font::{ascii::FONT_7X13 as INTRO_FONT, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Baseline, Text},
};

use display_interface_i2c::I2CInterface;
use ssd1309::{
    displayrotation::DisplayRotation, mode::GraphicsMode, prelude::DisplaySize, Builder,
};
use {defmt_rtt as _, panic_probe as _};

mod formatter;
mod progressbar;

const I2C0_FREQUENCY_HZ: u32 = 400_000;

// Bind interrupts to the handlers inside embassy.
bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

#[embassy_executor::main]
async fn main(_s: Spawner) {
    let p = embassy_rp::init(Config::default());

    let mut i2c0_config = i2c::Config::default();
    i2c0_config.frequency = I2C0_FREQUENCY_HZ;

    let i2c0_sda = p.PIN_0;
    let i2c0_scl = p.PIN_1;

    let mut oled_reset = Output::new(p.PIN_4, Level::Low);

    let mut led = Output::new(p.PIN_25, Level::Low);
    let mut watchdog = Watchdog::new(p.WATCHDOG);

    let i2c0_bus = i2c::I2c::new_async(p.I2C0, i2c0_scl, i2c0_sda, Irqs, i2c0_config);
    let display_interface = I2CInterface::new(i2c0_bus, 0x3C, 0x40);
    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .with_rotation(DisplayRotation::Rotate0)
        .connect(display_interface)
        .into();

    let mut delay = embassy_time::Delay;

    display.reset(&mut oled_reset, &mut delay).unwrap();

    display.init().expect("Display connected?");

    display.clear();
    display.flush().unwrap();

    const INTRO_STYLE: MonoTextStyle<'_, BinaryColor> =
        MonoTextStyle::new(&INTRO_FONT, BinaryColor::On);

    Text::with_alignment(
        "Hi there",
        Point::new(64, 22),
        INTRO_STYLE,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();
    Text::with_alignment(
        "let's test display",
        Point::new(64, 52),
        INTRO_STYLE,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();
    display.flush().unwrap();

    Timer::after_millis(400).await;

    let mut pb = ProgressBar::new(10, 35, 108, 10);
    let mut index = 0u64;
    loop {
        display.clear();
        let progress = (index % 100) as f32 * (1.0 / 100.0);
        if let Err(e) = pb.draw(progress, &mut display) {
            on_bus_error(&mut watchdog, &mut led).await;
        }

        let mut buf = Formatter::<96>::new();
        write!(buf, "i: {}", index).unwrap();
        if let Err(e) =
            Text::with_baseline(buf.as_str(), Point::new(8, 8), INTRO_STYLE, Baseline::Top)
                .draw(&mut display)
        {
            on_bus_error(&mut watchdog, &mut led).await;
        }

        if let Err(e) = display.flush() {
            on_bus_error(&mut watchdog, &mut led).await;
        }

        index += 1;
    }
}

async fn on_bus_error(wdg: &mut Watchdog, led: &mut Output<'static, PIN_25>) -> ! {
    led.set_high();
    Timer::after_millis(100).await;
    led.set_low();
    wdg.trigger_reset();
    unreachable!("Watchdog triggered");
}
