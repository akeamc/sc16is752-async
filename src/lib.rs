#![no_std]

use core::{error::Error as CoreError, fmt};

use embedded_hal_async::{
    digital::Wait,
    spi::{Error as SpiError, SpiDevice},
};
use embedded_io_async::{ErrorKind, ErrorType, Read, Write};
use heapless::Vec;

pub use crate::low_level::Channel;
use crate::low_level::{FifoControl, RegisterWrapper, THR};

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
        // First enable FIFO - this is critical for TXLVL to work properly
        self.regs
            .write_fcr(
                self.channel,
                FifoControl::new()
                    .with_enable(true)
                    .with_reset_tx(true)
                    .with_reset_rx(true),
            )
            .await?;

        // Read current LCR to preserve settings
        let mut lcr_val = self.regs.read(low_level::LCR, self.channel).await?[0];

        // Enable divisor latch (set bit 7)
        lcr_val |= 0x80;
        self.regs
            .write(low_level::LCR, self.channel, [lcr_val])
            .await?;

        // Check MCR register to determine prescaler (like reference implementation)
        let mcr = self.regs.read(low_level::MCR, self.channel).await?[0];
        let prescaler = if mcr == 0 { 1 } else { 4 };

        // Calculate and write divisor
        let divisor = ((crystal_freq / prescaler) / (16 * baud_rate)) as u16;
        let [msb, lsb] = divisor.to_be_bytes();

        self.regs.write(low_level::DLL, self.channel, [lsb]).await?;
        self.regs.write(low_level::DLH, self.channel, [msb]).await?;

        // Configure line control: 8N1 (8 data bits, no parity, 1 stop bit)
        // Clear divisor latch enable (bit 7) and set 8-bit word length (bits 1:0 = 11)
        lcr_val = 0x03; // 8 data bits, no parity, 1 stop bit
        self.regs
            .write(low_level::LCR, self.channel, [lcr_val])
            .await?;

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

        self.regs.write_many_thr(self.channel, buf).await?;

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
