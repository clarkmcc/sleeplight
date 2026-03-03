#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use bt_hci::controller::ExternalController;
use embassy_time::{Duration, Timer};
use core::str::FromStr;
use embassy_executor::Spawner;
use embassy_futures::join::{join, join3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::peripherals::{ADC1, RNG};
use esp_hal::rmt::Rmt;
use esp_hal::rng::{Trng, TrngSource};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ble::controller::BleConnector;
use firmware::lightring::{LightRing, LightState};
use rtt_target::rprintln;
use trouble_host::prelude::service::{BATTERY, DEVICE_INFORMATION, GATT};
use trouble_host::prelude::*;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rprintln!("PANIC: {:?}", info);
    loop {}
}

pub enum LightCommand {
    SetState(LightState)
}

static LIGHT_SIGNAL: Channel<CriticalSectionRawMutex, LightCommand, 8> = Channel::new();

extern crate alloc;

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

const NAME: &str = "SleepLight";

const BLE_SELF_UUID: u128 = 0xf3e0c0018b6f4d2ea2d06b9c3f2a0000u128;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    rtt_target::rtt_init_print!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // let mut adc1_config = AdcConfig::new();
    // let mut level_pin = adc1_config.enable_pin(peripherals.GPIO3, Attenuation::_11dB);
    // let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);

    // let mut raw: u16 = 0;
    // for _ in 0..5 {
    //     raw = nb::block!(adc1.read_oneshot(&mut level_pin)).unwrap();
    //     let voltage_mv = (raw as u32 * 3300 * 2) / 4095;
    //     rprintln!("Voltage: {} {}", voltage_mv, raw);
    //     Timer::after(Duration::from_millis(100)).await;
    // }

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    rprintln!("Embassy initialized!");

    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(&radio_init, peripherals.BT, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 20>::new(transport);
    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let _ = spawner;

    let rmt: Rmt<'_, esp_hal::Blocking> = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .expect("failed to initialize rmt");

    let ring = LightRing::new(peripherals.GPIO7, rmt.channel1);

    // Pull GPIO10 high to turn on the boost converter
    let _boost = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());

    // loop {
    //     ring.set_color(Color::Red).await;
    //     Timer::after(Duration::from_millis(100)).await;
    //     ring.set_color(Color::Green).await;
    //     Timer::after(Duration::from_millis(100)).await;
    // }

    let stat1 = Input::new(peripherals.GPIO4, InputConfig::default());
    let stat2 = Input::new(peripherals.GPIO5, InputConfig::default());

    BleServer::run(ble_controller, &ring, &mut resources, &stat1, &stat2).await;
    loop {
        rprintln!("in a loop")
    }

    // let mut color = Hsv {
    //     hue: 0,
    //     sat: 255,
    //     val: 255,
    // };
    // let mut data: RGB8;
    // let level = 10;
    //
    // loop {
    //     // led.write(brightness(gamma([RGB8::new(255, 0, 0); 8].into_iter()), 10)).await.unwrap();
    //     // Timer::after(Duration::from_millis(30)).await;
    //     // led.write(brightness(gamma([RGB8::new(255, 0, 0); 8].into_iter()), 0)).await.unwrap();
    //     // Timer::after(Duration::from_millis(30)).await;
    //     for hue in 0..=255u8 {
    //         color.hue = hue;
    //         data = hsv2rgb(color);
    //         if let Err(e) = led.write(brightness(gamma([data; 8].into_iter()), level)).await {
    //             rprintln!("LED write error at hue {}: {:?}", hue, e);
    //         }
    //         Timer::after(Duration::from_millis(10)).await;
    //     }
    // }

    // loop {
    //     rprintln!("Hello world!");
    //     Timer::after(Duration::from_secs(1)).await;
    // }
}

#[gatt_server]
pub struct BleServer {
    generic_access: GenericAccess,
    battery_service: BatteryService,
    light_service: LightService,
}

#[gatt_service(uuid = GATT)]
struct GenericAccess {
    #[characteristic(uuid = characteristic::DEVICE_NAME, read, value = HeaplessString::from_str(NAME).unwrap())]
    device_name: HeaplessString<32>,
    #[characteristic(uuid = characteristic::APPEARANCE, read, value = appearance::UNKNOWN)]
    appearance: BluetoothUuid16,
}

#[gatt_service(uuid = BATTERY)]
struct BatteryService {
    #[descriptor(uuid = descriptors::VALID_RANGE, read, value = [0, 100])]
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, value = "Battery Level")]
    #[characteristic(uuid = characteristic::BATTERY_LEVEL, read, write, notify, value = 10)]
    level: u8,
}

const LIGHT_SERVICE: u128 = 0xf3e0c0018b6f4d2ea2d06b9c3f2a0000u128;

#[gatt_service(uuid = LIGHT_SERVICE)]
struct LightService {
    #[descriptor(uuid = descriptors::CHARACTERISTIC_USER_DESCRIPTION, value = "State")]
    #[characteristic(uuid = "f3e0c002-8b6f-4d2e-a2d0-6b9c3f2a0000", read, write, notify, value = 10)]
    state: u32,
}

impl BleServer<'_> {
    pub async fn run<C>(
        controller: C,
        ring: &LightRing<'static>,
        resources: &mut HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>,
        stat1: &Input<'static>,
        stat2: &Input<'static>,
    ) where
        C: Controller,
    {
        let mut buf = [0u8; 6];
        let trng = Trng::try_new().unwrap();
        trng.read(&mut buf);
        let address: Address = Address::random(buf);

        let stack = trouble_host::new(controller, resources).set_random_address(address);
        let host = stack.build();
        let mut peripheral = host.peripheral;

        let mut server = BleServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: NAME,
            appearance: &appearance::light_source::GENERIC_LIGHT_SOURCE,
        }))
        .expect("create ble server");

        server
            .set(&server.battery_service.level, &70u8)
            .expect("TODO: panic message");

        join3(
            ble_runner(host.runner),
            ble_advertise_loop(&mut peripheral, &mut server, &ring),
            async {
                loop {
                    match (stat1.level(), stat2.level()) {
                        (Level::High, Level::High) => {
                            rprintln!("[bat] shutdown/standby");
                        }
                        (Level::High, Level::Low) => {
                            rprintln!("[bat] charge complete");
                        }
                        (Level::Low, Level::High) => {
                            rprintln!("[bat] preconditioning/fast charge");
                        }
                        (Level::Low, Level::Low) => {
                            rprintln!("[bat] temp/timer fault");
                        }
                        _ => {}
                    }
                    Timer::after(Duration::from_millis(1000)).await;
                }
            }
        )
        .await;
    }
}

async fn ble_runner<C: Controller>(mut runner: Runner<'_, C, DefaultPacketPool>) {
    loop {
        if let Err(e) = runner.run().await {
            panic!("[ble_task] error: {:?}", e);
        }
    }
}

async fn ble_advertise_loop<'ble, C>(
    peripheral: &mut Peripheral<'ble, C, DefaultPacketPool>,
    server: &mut BleServer<'ble>,
    ring: &LightRing<'static>,
) where
    C: Controller,
{
    loop {
        match advertise(peripheral, server).await {
            Ok(conn) => {
                if let Err(err) = ble_handle_conn(server, &conn, &ring).await {
                    rprintln!("[ble_advertise_loop] error handling connection: {:?}", err);
                }
            }
            Err(err) => {
                rprintln!("[ble_advertise_loop] err: {:?}", err);
            }
        }
    }
}

async fn advertise<'values, 'server, C: Controller>(
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server BleServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    // Split advertisement data between adv_data and scan_data to fit within 31-byte limits
    let mut advertiser_data = [0; 31];
    let adv_len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[BATTERY.to_le_bytes()]), // Keep only essential service in adv_data
            AdStructure::CompleteLocalName(NAME.as_bytes()),
        ],
        &mut advertiser_data,
    )?;

    // Put additional services in scan response data
    let mut scan_response_data = [0; 31];
    let scan_len = AdStructure::encode_slice(
        &[
            AdStructure::ServiceUuids16(&[DEVICE_INFORMATION.to_le_bytes(), GATT.to_le_bytes()]),
            AdStructure::ServiceUuids128(&[BLE_SELF_UUID.to_le_bytes()]),
        ],
        &mut scan_response_data,
    )?;

    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &advertiser_data[..adv_len],
                scan_data: &scan_response_data[..scan_len],
            },
        )
        .await?;
    rprintln!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    rprintln!("[adv] connected");
    Ok(conn)
}

async fn ble_handle_conn(server: &BleServer<'_>, conn: &GattConnection<'_, '_, DefaultPacketPool>, ring: &LightRing<'static>) -> Result<(), Error> {
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(event) => {
                    }
                    GattEvent::Write(event) => {
                        rprintln!("[ble_handle_conn] event: handle={},data={:?}", event.handle(), event.data());
                        if event.handle() == server.light_service.state.handle {
                            let value = LightState::from_ble_u32(event.value::<u32>(&server.light_service.state).expect("value"));
                            rprintln!("[ble_handle_conn] setting light state: {:?}", value);
                            ring.set(value);
                        }
                    }
                    _ => {}
                }
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => rprintln!("[gatt] Error sending response: {:?}", e),
                }
            }
            GattConnectionEvent::PhyUpdated { .. } => {}
            GattConnectionEvent::ConnectionParamsUpdated { .. } => {}
            GattConnectionEvent::RequestConnectionParams { .. } => {}
            GattConnectionEvent::DataLengthUpdated { .. } => {}
        }
    };
    rprintln!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

struct BatteryStatus {
    stat1: Input<'static>,
    stat2: Input<'static>,
}

impl BatteryStatus {
    fn new(stat1: Input<'static>, stat2: Input<'static>) -> Self {
        BatteryStatus { stat1, stat2 }
    }
}

impl From<BatteryStatus> for u8 {
    fn from(status: BatteryStatus) -> Self {
        let level1 = status.stat1.level();
        let level2 = status.stat2.level();
        match (level1, level2) {
            (Level::High, Level::High) => 1,
            (Level::High, Level::Low) => 2,
            (Level::Low, Level::High) => 3,
            (Level::Low, Level::Low) => 4,
            _ => 0,
        }
    }
}
