
pub struct App {
    /// The brightness of the LEDs on a scale from 0-255. Should stay under 50
    /// for current limiting reasons.
    brightness: u8,

    /// The color of the LEDs as an RGB tuple on a scale of 0-255.
    color: [u8; 3], // rgb
}

impl App {

}