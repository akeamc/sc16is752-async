#![no_std]

use embedded_hal_async::{
    digital::Wait,
    spi::{Error as SpiError, SpiDevice},
};
use embedded_io_async::{ErrorKind, ErrorType, Read, Write};

pub use crate::low_level::Channel;
use crate::low_level::{FifoControl, Ier, RegisterWrapper};

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
        // Enable FIFO with reset of both TX and RX
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

        // Check MCR register to determine prescaler
        let mcr = self.regs.read(low_level::MCR, self.channel).await?[0];
        let prescaler = if mcr == 0 { 1 } else { 4 };

        // Calculate and write divisor
        let divisor = ((crystal_freq / prescaler) / (16 * baud_rate)) as u16;
        let [msb, lsb] = divisor.to_be_bytes();

        self.regs.write(low_level::DLL, self.channel, [lsb]).await?;
        self.regs.write(low_level::DLH, self.channel, [msb]).await?;

        // Configure line control: 8N1 (8 data bits, no parity, 1 stop bit)
        self.regs
            .write(low_level::LCR, self.channel, [0x03])
            .await?;

        // Enable RHR interrupt so we get notified when data arrives
        self.regs
            .write_ier(self.channel, Ier::new().with_receive_holding_register(true))
            .await?;

        Ok(())
    }

    async fn wait_for_irq(&mut self) -> Result<(), Error<Spi::Error>> {
        self.irq.wait_for_low().await.ok();
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<SpiErr> {
    #[error("spi error: {0:?}")]
    Spi(SpiErr),
}

impl<SpiErr: SpiError> embedded_io_async::Error for Error<SpiErr> {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        ErrorKind::Other
    }
}

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

        loop {
            let space = self.regs.read_txlvl(self.channel).await? as usize;

            if space > 0 {
                let len = buf.len().min(space);
                self.regs.write_many_thr(self.channel, &buf[..len]).await?;
                return Ok(len);
            }

            // No space — enable THR interrupt and wait
            self.regs
                .write_ier(
                    self.channel,
                    Ier::new()
                        .with_receive_holding_register(true)
                        .with_transmit_holding_register(true),
                )
                .await?;

            self.wait_for_irq().await?;

            // Read IIR to clear the interrupt
            let _iir = self.regs.read_iir(self.channel).await?;

            // Disable THR interrupt (leave RHR enabled)
            self.regs
                .write_ier(self.channel, Ier::new().with_receive_holding_register(true))
                .await?;
        }
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        loop {
            let lsr = self.regs.read_lsr(self.channel).await?;
            if lsr.thr_empty() && lsr.thr_tsr_empty() {
                return Ok(());
            }

            // Enable THR interrupt and wait for FIFO to drain
            self.regs
                .write_ier(
                    self.channel,
                    Ier::new()
                        .with_receive_holding_register(true)
                        .with_transmit_holding_register(true),
                )
                .await?;

            self.wait_for_irq().await?;
            let _iir = self.regs.read_iir(self.channel).await?;

            // Disable THR interrupt
            self.regs
                .write_ier(self.channel, Ier::new().with_receive_holding_register(true))
                .await?;
        }
    }
}

impl<Spi: SpiDevice, Irq: Wait> Read for Sc16is752<Spi, Irq> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        loop {
            let available = self.regs.read_rxlvl(self.channel).await? as usize;

            if available > 0 {
                let len = buf.len().min(available);
                self.regs
                    .read_many_rhr(self.channel, &mut buf[..len])
                    .await?;
                return Ok(len);
            }

            // No data — wait for RHR interrupt
            self.wait_for_irq().await?;

            // Read IIR to clear the interrupt
            let _iir = self.regs.read_iir(self.channel).await?;
        }
    }
}
