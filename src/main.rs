#![cfg_attr(not(test), no_std)]
#![no_main]
#![feature(type_alias_impl_trait)]
#![forbid(unsafe_code)]
#![allow(rustdoc::bare_urls)]

use crate::progressbar::ProgressBar;
use core::fmt::Write;
use defmt::{info, trace};
use embassy_executor::Executor;
use embassy_rp::{
    config::Config,
    gpio::{Input, Level, Output, Pull},
    multicore::{spawn_core1, Stack},
    peripherals::{PIN_16, PIN_17, PIN_20, PIN_25, PIN_26, PIN_5, SPI0},
    spi::{self, Phase, Polarity, Spi},
    watchdog::Watchdog,
};
use embassy_time::Timer;
use embedded_graphics::{
    geometry::Point,
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    text::{Alignment, Text},
};

use crate::formatter::Formatter;
use display_interface_spi::SPIInterface;
use ssd1309::{
    displayrotation::DisplayRotation, mode::GraphicsMode, prelude::DisplaySize, Builder,
};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

pub const CORE1_STACK_SIZE: usize = 65_536;
static CORE1_STACK: StaticCell<Stack<CORE1_STACK_SIZE>> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();

mod formatter;
mod progressbar;

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("Hi display demo");
    let p = embassy_rp::init(Config::default());

    let mosi = p.PIN_19;
    let clk = p.PIN_18;
    let cs = p.PIN_17;
    let dc = Output::new(p.PIN_16, Level::Low);

    // create SPI
    let mut config = spi::Config::default();
    config.frequency = 2_000_000; //dof
    config.phase = Phase::CaptureOnFirstTransition;
    config.polarity = Polarity::IdleLow;
    let spi = Spi::new_blocking_txonly(p.SPI0, clk, mosi, config);

    // Configure CS
    let cs = Output::new(cs, Level::Low); //dof

    let button = Input::new(p.PIN_5, Pull::None);

    let mut oled_reset = Output::new(p.PIN_4, Level::Low);

    let led = Output::new(p.PIN_25, Level::Low);
    let watchdog = Watchdog::new(p.WATCHDOG);

    let mut delay = embassy_time::Delay;
    let display_interface = SPIInterface::new(spi, dc, cs);
    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .with_rotation(DisplayRotation::Rotate0)
        .connect(display_interface)
        .into();
    display
        .reset(&mut oled_reset, &mut embassy_time::Delay)
        .unwrap();

    display.reset(&mut oled_reset, &mut delay).unwrap();

    display.init().expect("Display connected?");
    info!("Display connected");

    display.clear();
    info!("Display cleared");
    display.flush().unwrap();
    info!("Display flushed");

    spawn_core1(p.CORE1, CORE1_STACK.init(Stack::new()), move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| spawner.spawn(progress(display, button, watchdog)).unwrap())
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| spawner.spawn(blinky(led)).unwrap());
}

pub use embedded_graphics::{
    image::Image,
    mono_font::{
        ascii::FONT_10X20 as LOGO_FONT, ascii::FONT_7X13 as INTRO_FONT,
        iso_8859_7::FONT_6X12 as RESULT_FONT,
    },
    prelude::*,
    text::Baseline,
};
pub const RESULT_STYLE: MonoTextStyle<'_, BinaryColor> =
    MonoTextStyle::new(&RESULT_FONT, BinaryColor::On);

#[embassy_executor::task]
async fn progress(
    mut display: GraphicsMode<
        SPIInterface<
            Spi<'static, SPI0, spi::Blocking>,
            Output<'static, PIN_16>,
            Output<'static, PIN_17>,
        >,
    >,
    button: Input<'static, PIN_5>,
    mut watchdog: Watchdog,
) -> ! {
    info!("progress");
    let mut pb = ProgressBar::new(10, 35, 108, 10);
    let mut index = 0u64;
    let mut buffer = Formatter::<6>::new();
    loop {
        info!("clearing");
        display.clear();
        info!("cleared");

        let times = index / 100;
        write!(buffer, "{times}").unwrap();
        Text::with_alignment(
            buffer.as_str(),
            Point::new(64, 15),
            RESULT_STYLE,
            Alignment::Center,
        )
        .draw(&mut display)
        .unwrap();
        buffer.clear();
        let progress = (index % 100) as f32 * (1.0 / 100.0);
        trace!("loop {}", progress);
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
