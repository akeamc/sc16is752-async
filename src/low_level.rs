use embedded_hal_async::spi::SpiDevice;
use modular_bitfield::prelude::*;

use crate::Error;

pub struct RegisterWrapper<Spi> {
    spi: Spi,
}

// Register addresses matching reference implementation
pub const THR: u8 = 0x00; // RhrThr register
pub const IER: u8 = 0x01;
pub const FCR: u8 = 0x02; // FcrIir register
pub const LCR: u8 = 0x03;
pub const MCR: u8 = 0x04;
pub const LSR: u8 = 0x05;
pub const DLL: u8 = 0x00; // Same as THR when LCR[7]=1
pub const DLH: u8 = 0x01; // Same as IER when LCR[7]=1
pub const TXLVL: u8 = 0x08;
pub const RXLVL: u8 = 0x09;
pub const IIR: u8 = 0x02;
pub const RHR: u8 = 0x00;

impl<Spi: SpiDevice> RegisterWrapper<Spi> {
    pub fn new(spi: Spi) -> Self {
        RegisterWrapper { spi }
    }

    pub async fn write(
        &mut self,
        reg: u8,
        channel: Channel,
        value: [u8; 1],
    ) -> Result<(), Error<Spi::Error>> {
        let [rab] = Rab::new()
            .with_rw(ReadWrite::Write)
            .with_register(reg)
            .with_channel(channel)
            .into_bytes();

        self.spi
            .write(&mut [rab, value[0]])
            .await
            .map_err(Error::Spi)
    }

    pub async fn read(&mut self, reg: u8, channel: Channel) -> Result<[u8; 1], Error<Spi::Error>> {
        let [rab] = Rab::new()
            .with_rw(ReadWrite::Read)
            .with_register(reg)
            .with_channel(channel)
            .into_bytes();
        let mut buf = [rab, 0x00];

        self.spi
            .transfer_in_place(&mut buf)
            .await
            .map_err(Error::Spi)?;

        Ok([buf[1]])
    }

    pub async fn read_iir(&mut self, channel: Channel) -> Result<Iir, Error<Spi::Error>> {
        self.read(IIR, channel).await.map(Iir::from_bytes)
    }

    pub async fn write_ier(&mut self, channel: Channel, ier: Ier) -> Result<(), Error<Spi::Error>> {
        self.write(IER, channel, ier.into_bytes()).await
    }

    pub async fn write_fcr(
        &mut self,
        channel: Channel,
        fcr: FifoControl,
    ) -> Result<(), Error<Spi::Error>> {
        self.write(FCR, channel, fcr.into_bytes()).await
    }

    pub async fn write_lcr(
        &mut self,
        channel: Channel,
        lcr: LineControl,
    ) -> Result<(), Error<Spi::Error>> {
        self.write(LCR, channel, lcr.into_bytes()).await
    }

    pub async fn write_mcr(
        &mut self,
        channel: Channel,
        mcr: ModemControl,
    ) -> Result<(), Error<Spi::Error>> {
        self.write(MCR, channel, mcr.into_bytes()).await
    }

    pub async fn write_divisor(
        &mut self,
        channel: Channel,
        divisor: u16,
    ) -> Result<(), Error<Spi::Error>> {
        let [msb, lsb] = divisor.to_be_bytes();

        self.write(DLL, channel, [lsb]).await?;
        self.write(DLH, channel, [msb]).await
    }

    pub async fn read_txlvl(&mut self, channel: Channel) -> Result<u8, Error<Spi::Error>> {
        self.read(TXLVL, channel).await.map(|[byte]| byte)
    }

    pub async fn read_rxlvl(&mut self, channel: Channel) -> Result<u8, Error<Spi::Error>> {
        self.read(RXLVL, channel).await.map(|[byte]| byte)
    }
}

#[derive(Specifier, Debug, Clone, Copy)]
#[bits = 2] // sic!
pub enum Channel {
    A = 0b00,
    B = 0b01,
}

#[derive(Specifier, Debug)]
#[bits = 1]
enum ReadWrite {
    Write = 0,
    Read = 1,
}

#[bitfield(bits = 8)]
struct Rab {
    #[skip]
    unused: B1,
    channel: Channel,
    register: B4,
    rw: ReadWrite,
}

#[derive(Specifier)]
#[bits = 5]
pub enum InterruptSource {
    ReceiveLineStatusError = 0b00011,
    ReceiverTimeout = 0b00110,
    RhrInterrupt = 0b00010,
    ThrInterrupt = 0b00001,
    ModemInterrupt = 0b00000,
}

#[bitfield(bits = 8)]
pub struct Iir {
    fcr_msb: B2,
    pub source: InterruptSource,
    pub pending: bool,
}

#[bitfield(bits = 8)]
pub struct Ier {
    pub cts: bool,
    pub rts: bool,
    pub x_off: bool,
    pub sleep: bool,
    pub modem_status: bool,
    pub receive_line_status: bool,
    pub transmit_holding_register: bool,
    pub receive_holding_register: bool,
}

#[derive(Specifier)]
pub enum RxFifoTrigger {
    _8 = 0b00,
    _16 = 0b01,
    _56 = 0b10,
    _60 = 0b11,
}

#[derive(Specifier)]
pub enum TxFifoTrigger {
    _8 = 0b00,
    _16 = 0b01,
    _32 = 0b10,
    _56 = 0b11,
}

#[bitfield(bits = 8)]
pub struct FifoControl {
    pub rx_trigger: RxFifoTrigger,
    pub tx_trigger: TxFifoTrigger,
    #[skip]
    reserved: B1,
    pub reset_tx: bool,
    pub reset_rx: bool,
    pub enable: bool,
}

#[bitfield(bits = 8)]
#[derive(Debug, Clone, Copy)]
pub struct LineControl {
    pub divisor_latch_enable: bool,
    pub break_control_bit: bool,
    #[skip]
    unused: B6,
}

#[derive(Specifier)]
pub enum Divisor {
    DivideByOne = 0,
    DivideByFour = 1,
}

#[bitfield(bits = 8)]
pub struct ModemControl {
    divisor: Divisor,
    unused: B7,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rab_construction() {
        // Test TXLVL read on Channel A - should be 0xC0
        let rab = Rab::new()
            .with_rw(ReadWrite::Read)
            .with_register(TXLVL)
            .with_channel(Channel::A);
        assert_eq!(rab.into_bytes()[0], 0xC0);

        // Test IER write on Channel A - should be 0x08
        let rab = Rab::new()
            .with_rw(ReadWrite::Write)
            .with_register(IER)
            .with_channel(Channel::A);
        assert_eq!(rab.into_bytes()[0], 0x08);

        // Test TXLVL read on Channel B - should be 0xC2
        let rab = Rab::new()
            .with_rw(ReadWrite::Read)
            .with_register(TXLVL)
            .with_channel(Channel::B);
        assert_eq!(rab.into_bytes()[0], 0xC2);
    }
}
