#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_io_async::{Read, Write};
use esp_hal::gpio::{Input, Level, Output};
use esp_hal::spi::master::Config;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, spi::master::Spi};
use sc16is752_async::{Channel, Sc16is752};
use {esp_backtrace as _, esp_println as _};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    // generator version: 1.2.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    let spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_mhz(4)),
    )
    .unwrap()
    .with_mosi(peripherals.GPIO23)
    .with_miso(peripherals.GPIO19)
    .with_sck(peripherals.GPIO18)
    .into_async();

    let cs = Output::new(peripherals.GPIO12, Level::High, Default::default());
    let irq = Input::new(peripherals.GPIO14, Default::default());

    let Ok(spi_dev) = ExclusiveDevice::new(spi_bus, cs, embassy_time::Delay);

    let mut sc16 = Sc16is752::new(spi_dev, irq, Channel::A);

    const BAUD: u32 = 115_200;
    const XTAL_FREQ: u32 = 14_745_600;

    sc16.init(BAUD, XTAL_FREQ).await.unwrap();

    loop {
        sc16.write_all(b"Hello world!").await.unwrap();
        let mut buf = [0; 12];
        sc16.read_exact(&mut buf).await.unwrap();

        info!("Read: {:a}", buf);

        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
