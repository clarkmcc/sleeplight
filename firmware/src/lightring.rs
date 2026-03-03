use core::cell::RefCell;
use esp_hal::Blocking;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::rmt::{PulseCode, TxChannelCreator};
use esp_hal_smartled::{buffer_size_async, SmartLedsAdapter};
use smart_leds::{brightness, gamma, RGB8, SmartLedsWrite};
use static_cell::StaticCell;

pub const NUM_LEDS: usize = 8;
const BUFFER_SIZE: usize = buffer_size_async(NUM_LEDS);
static BUFFER: StaticCell<[PulseCode; BUFFER_SIZE]> = StaticCell::new();

struct LightRingInner<'rmt> {
    adapter: SmartLedsAdapter<'rmt, BUFFER_SIZE>,
    brightness: u8,
    color: Option<RGB8>,
}

pub struct LightRing<'rmt> {
    inner: RefCell<LightRingInner<'rmt>>,
}

impl<'rmt> LightRing<'rmt> {
    pub fn new<P: PeripheralOutput<'rmt>, C: TxChannelCreator<'rmt, Blocking>>(pin: P, rmt_channel: C) -> Self {
        let buffer = BUFFER.init([PulseCode::end_marker(); BUFFER_SIZE]);
        LightRing {
            inner: RefCell::new(LightRingInner {
                adapter: SmartLedsAdapter::new(rmt_channel, pin, buffer),
                brightness: 10,
                color: None,
            }),
        }
    }

    pub fn set(&self, state: LightState) {
        let mut inner = self.inner.borrow_mut();
        inner.color = Some(state.to_rgb8());
        inner.brightness = state.brightness();
        Self::update(&mut inner);
    }

    pub fn off(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.color = None;
        Self::update(&mut inner);
    }

    fn update(inner: &mut LightRingInner<'rmt>) {
        let color = inner.color.unwrap_or_default();
        let pixels = brightness(gamma([color; NUM_LEDS].into_iter()), inner.brightness);
        inner.adapter.write(pixels).expect("write");
    }
}

/// Packs red, green, blue, and brightness into a single `u32`.
///
/// Bit layout (MSB → LSB): `[red: 8][green: 8][blue: 8][brightness: 8]`
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LightState(pub u32);

impl core::fmt::Debug for LightState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LightState")
            .field("r", &self.r())
            .field("g", &self.g())
            .field("b", &self.b())
            .field("brightness", &self.brightness())
            .finish()
    }
}

impl LightState {
    pub fn new(r: u8, g: u8, b: u8, brightness: u8) -> Self {
        Self(
            (r as u32) << 24
                | (g as u32) << 16
                | (b as u32) << 8
                | brightness as u32,
        )
    }

    /// Constructs a `LightState` from a raw `u32` received over BLE.
    ///
    /// BLE/ATT transmits multi-byte integers in little-endian byte order, so the
    /// wire bytes must be re-interpreted as big-endian to match the packed layout.
    pub fn from_ble_u32(le_value: u32) -> Self {
        Self(u32::from_be_bytes(le_value.to_le_bytes()))
    }

    pub fn r(self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub fn g(self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn b(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn brightness(self) -> u8 {
        (self.0 as u8).clamp(0, 50)
    }

    pub fn to_rgb8(self) -> RGB8 {
        RGB8 { r: self.r(), g: self.g(), b: self.b() }
    }
}