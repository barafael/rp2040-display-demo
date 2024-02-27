#![cfg_attr(not(test), no_std)]
#![no_main]
#![feature(type_alias_impl_trait)]
#![forbid(unsafe_code)]
#![allow(rustdoc::bare_urls)]

use core::fmt::Write;

use crate::progressbar::ProgressBar;

use defmt::info;
use display_interface_spi::SPIInterface;
use embassy_executor::Executor;
use embassy_rp::{
    config::Config,
    gpio::{Level, Output},
    multicore::{spawn_core1, Stack},
    peripherals::{PIN_0, PIN_1, PIN_25, SPI0},
    spi::{self, Phase, Polarity, Spi},
    watchdog::Watchdog,
};
use embassy_time::Timer;
use embedded_graphics::{
    mono_font::{iso_8859_7::FONT_6X12, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};
use heapless::String;
use ssd1309::{
    displayrotation::DisplayRotation, mode::GraphicsMode, prelude::DisplaySize, Builder,
};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

pub const CORE1_STACK_SIZE: usize = 65_536;
static CORE1_STACK: StaticCell<Stack<CORE1_STACK_SIZE>> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();

pub const STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyle::new(&FONT_6X12, BinaryColor::On);

mod progressbar;

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("Hi display demo");
    let p = embassy_rp::init(Config::default());

    let led = Output::new(p.PIN_25, Level::Low);
    let watchdog = Watchdog::new(p.WATCHDOG);
    let mut oled_reset = Output::new(p.PIN_4, Level::Low);

    let mut config = spi::Config::default();
    config.frequency = 2_000_000;
    config.phase = Phase::CaptureOnFirstTransition;
    config.polarity = Polarity::IdleLow;

    let tx = p.PIN_3;
    let clk = p.PIN_2;
    let cs = Output::new(p.PIN_1, Level::Low);
    let dc = Output::new(p.PIN_0, Level::Low);
    let spi = Spi::new_blocking_txonly(p.SPI0, clk, tx, config);

    let display_interface = SPIInterface::new(spi, dc, cs);
    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .with_rotation(DisplayRotation::Rotate0)
        .connect(display_interface)
        .into();
    display
        .reset(&mut oled_reset, &mut embassy_time::Delay)
        .unwrap();
    display.init().expect("Display connected?");

    display.clear();
    display.flush().unwrap();

    spawn_core1(p.CORE1, CORE1_STACK.init(Stack::new()), move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| spawner.spawn(progress(display, watchdog)).unwrap())
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| spawner.spawn(blinky(led)).unwrap());
}

#[embassy_executor::task]
async fn progress(
    mut display: GraphicsMode<
        SPIInterface<
            Spi<'static, SPI0, spi::Blocking>,
            Output<'static, PIN_0>,
            Output<'static, PIN_1>,
        >,
    >,
    mut watchdog: Watchdog,
) -> ! {
    let mut pb = ProgressBar::new(10, 35, 108, 8);
    let mut index = 0u64;
    let mut buffer = String::<6>::new();
    loop {
        display.clear();

        let times = index / 100;
        write!(buffer, "{times}").unwrap();
        Text::with_alignment(
            buffer.as_str(),
            Point::new(64, 15),
            STYLE,
            Alignment::Center,
        )
        .draw(&mut display)
        .unwrap();
        buffer.clear();
        let progress = (index % 100) as f32 * (1.0 / 100.0);
        if let Err(_e) = pb.draw(progress, &mut display) {
            on_bus_error(&mut watchdog);
        }
        if let Err(_e) = display.flush() {
            on_bus_error(&mut watchdog);
        }
        index += 1;
    }
}

fn on_bus_error(wdg: &mut Watchdog) -> ! {
    wdg.trigger_reset();
    unreachable!("Watchdog triggered");
}

#[embassy_executor::task]
async fn blinky(mut led: Output<'static, PIN_25>) -> ! {
    loop {
        Timer::after_millis(400).await;
        led.toggle();
    }
}
