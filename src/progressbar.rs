use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
};

pub struct ProgressBar {
    x: u32,
    y: u32,
    length: u32,
    width: u32,
}

const STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .fill_color(BinaryColor::Off)
    .build();

const FILL_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .fill_color(BinaryColor::On)
    .build();

impl ProgressBar {
    pub fn new(x: u32, y: u32, length: u32, width: u32) -> Self {
        Self {
            x,
            y,
            length,
            width,
        }
    }

    pub fn draw<D>(&mut self, ratio: f32, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
        D::Error: core::fmt::Debug,
    {
        Rectangle::new(
            Point::new(self.x as i32, self.y as i32),
            Size::new(self.length, self.width),
        )
        .into_styled(STYLE)
        .draw(display)?;
        let fill = (self.length as f32 * ratio) as u32;
        Rectangle::new(
            Point::new(self.x as i32, self.y as i32),
            Size::new(fill, self.width),
        )
        .into_styled(FILL_STYLE)
        .draw(display)?;
        Ok(())
    }
}
