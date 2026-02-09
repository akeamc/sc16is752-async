#![no_std]

use core::{error::Error as CoreError, fmt};

use embedded_hal_async::{
    delay::DelayNs,
    digital::Wait,
    spi::{Error as SpiError, SpiDevice},
};
use embedded_io_async::{ErrorKind, ErrorType, Read, Write};
use log::info;
use modular_bitfield::prelude::*;

pub use crate::low_level::Channel;
use crate::low_level::{FifoControl, Ier, InterruptSource, LineControl, RegisterWrapper};

mod low_level;

pub struct Sc16is752<Spi, Irq> {
    regs: RegisterWrapper<Spi>,
    irq: Irq,
    channel: Channel,
}

impl<Spi, Irq> Sc16is752<Spi, Irq>
where
    Spi: SpiDevice,
    Irq: Wait,
{
    pub fn new(spi: Spi, irq: Irq, channel: Channel) -> Self {
        Sc16is752 {
            regs: RegisterWrapper::new(spi),
            irq,
            channel,
        }
    }

    pub async fn init(
        &mut self,
        baud_rate: u32,
        crystal_freq: u32,
    ) -> Result<(), Error<Spi::Error>> {
        // Enable FIFO and reset TX/RX FIFOs
        self.regs
            .write_fcr(
                self.channel,
                FifoControl::new()
                    .with_enable(true)
                    .with_reset_tx(true)
                    .with_reset_rx(true),
            )
            .await?;

        let lcr = LineControl::new();

        // Enable divisor latch to set baud rate
        self.regs
            .write_lcr(self.channel, lcr.with_divisor_latch_enable(true))
            .await?;

        self.regs
            .write_divisor(self.channel, (crystal_freq / (16 * baud_rate)) as u16)
            .await?;

        // Disable divisor latch to return to normal operation
        self.regs
            .write_lcr(self.channel, lcr.with_divisor_latch_enable(false))
            .await?;

        // self.regs
        //     .write_ier(
        //         self.channel,
        //         Ier::new().with_transmit_holding_register(true),
        //     )
        //     .await?;

        Ok(())
    }

    async fn wait_for_irq(&mut self) {
        self.irq.wait_for_low().await.unwrap();
    }
}

#[derive(Debug)]
pub enum Error<SpiErr> {
    Spi(SpiErr),
}

impl<SpiErr: SpiError> embedded_io_async::Error for Error<SpiErr> {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        ErrorKind::Other
    }
}

impl<SpiErr: SpiError> fmt::Display for Error<SpiErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SC16IS752 Error: {}", self)
    }
}

impl<SpiErr: SpiError> CoreError for Error<SpiErr> {}

impl<Spi, Irq> ErrorType for Sc16is752<Spi, Irq>
where
    Spi: embedded_hal_async::spi::ErrorType,
{
    type Error = Error<Spi::Error>;
}

impl<Spi: SpiDevice, Irq: Wait> Write for Sc16is752<Spi, Irq> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        // Get available space in TX FIFO
        let space_left = self.regs.read_txlvl(self.channel).await? as usize;
        let len = buf.len().min(space_left);

        info!("writing {} bytes", len);

        // Write bytes to THR register
        for i in 0..len {
            self.regs
                .write(low_level::THR, self.channel, [buf[i]])
                .await?;
        }

        Ok(len)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<Spi: SpiDevice, Irq: Wait> Read for Sc16is752<Spi, Irq> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}
