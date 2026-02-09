//!
//! # Example
//!
//! ```
//!
//! let mut device = SC16IS752::new(SC16IS752i2c::new(SC16IS750_ADDRESS, i2c))?;
//! let mut device = SC16IS752::new(SC16IS752spi::new(spi_dev))?;
//!
//! device.initalise(Channel::A, UartConfig::default().baudrate(9600))?;
//! device.gpio_set_pin_mode(GPIO::GPIO0, PinMode::Output)?;
//! device.flush(Channel::A)?;
//! loop {
//!     device.write_byte(Channel::A, "a".as_bytes()[0])?;
//!     println!("RX A = {:?}", device.read_byte(Channel::A)?);
//!
//!     for byte in b"This is channel A" {
//!         device.write_byte(Channel::A, byte)?;
//!     }
//!     let mut buf_a: Vec<u8> = vec![];
//!     for _ in 0..device.fifo_available_data(Channel::A)? {
//!         match device.read_byte(Channel::A)? {
//!             Some(byte) => buf_a.push(byte),
//!             None => break,
//!         }
//!     }
//!     println!("RX UART A = {}", String::from_utf8_lossy(&buf_a));
//!
//!     thread::sleep(Duration::from_millis(250));
//!     println!("Testing GPIO states");
//!     device.gpio_set_pin_state(GPIO::GPIO0, PinState::High)?;
//!     println!(
//!         "GPIO0 = {:?} (Output)",
//!         device.gpio_get_pin_state(GPIO::GPIO0)?
//!     );
//!     println!("Set GPIO0 to low");
//!     device.gpio_set_pin_state(GPIO::GPIO0, PinState::Low)?;
//!     thread::sleep(Duration::from_millis(250));
//!     println!(
//!         "GPIO0 = {:?} (Output toggled)",
//!         device.gpio_get_pin_state(GPIO::GPIO0)?
//!     );
//! ```

#![no_std]

use embedded_hal::i2c::I2c;
use embedded_hal::spi::SpiDevice;

/// UARTs Channel A (TXA/RXA) and Channel B (TXB/RXB)
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Channel {
    /// UART A
    A,
    /// UART B
    B,
}
impl core::fmt::Display for Channel {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[repr(u8)]
pub enum Registers {
    /// UART A
    RhrThr = 0x00,
    IER = 0x01,
    FcrIir = 0x02,
    LCR = 0x03,
    MCR = 0x04,
    LSR = 0x05,
    MsrTcr = 0x06,
    SprTlr = 0x07,
    TXLVL = 0x08,
    RXLVL = 0x09,
    IODir = 0x0A,
    IOState = 0x0B,
    IOIntEna = 0x0C,
    IOControl = 0x0E,
    EFCR = 0x0F,
}

/// GPIO pins 0-7
#[allow(missing_docs)]
#[derive(Debug)]
pub enum GPIO {
    GPIO0,
    GPIO1,
    GPIO2,
    GPIO3,
    GPIO4,
    GPIO5,
    GPIO6,
    GPIO7,
}
impl core::fmt::Display for GPIO {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[allow(missing_docs)]
#[derive(Debug)]
pub enum PinMode {
    Input,
    Output,
}
impl core::fmt::Display for PinMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum PinState {
    Low,
    High,
}
impl core::fmt::Display for PinState {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[allow(non_camel_case_types)]
pub enum InterruptEventTest {
    RECEIVE_LINE_STATUS_ERROR,
    RECEIVE_TIMEOUT_INTERRUPT,
    RHR_INTERRUPT,
    THR_INTERRUPT,
    MODEM_INTERRUPT,
    INPUT_PIN_CHANGE_STATE,
    RECEIVE_XOFF,
    CTS_RTS_CHANGE,
    UNKNOWN,
}
/// Extra Features Control Register options
#[derive()]
pub enum FeaturesRegister {
    /// IrDaFast controls IrDA mode. (SC16IS762 only)
    ///
    /// false -> IrDA SIR, 3⁄16 pulse ratio, data rate up to 115.2 kbit/s
    /// true -> IrDA SIR, 1⁄4 pulse ratio, data rate up to 1.152 Mbit/s
    IrDaFast = 0x80,
    /// Inverts RTS signal in RS-485 mode
    ///
    /// false -> RTS pin = low during transmission and RTS pin = high during reception
    /// true -> RTS = high during transmission and RTS = low during reception
    AutoRs485RTSOutputInversion = 0x20,
    /// Enable the transmitter to control the RTS pin
    ///
    /// false -> transmitter does not control RTS pin
    /// true -> transmitter controls RTS pin
    AutoRs485DirectionControl = 0x10,
    /// Disables transmitter.
    ///
    /// UART does not send serial data out on the transmit pin, but the transmit FIFO will continue to receive data from host until full. Any data in the TSR will be sent out before the transmitter goes into disable state.
    TxDisable = 0x04,
    /// Disables receiver.
    ///
    /// UART will stop receiving data immediately once this is set to true, and any data in the TSR will be sent to the receive FIFO. User is advised not to set this bit during receiving.
    RxDisable = 0x02,
    /// Enable 9-bit/Multidrop mode (RS-485).
    Multidrop = 0x01,
}

#[allow(missing_docs)]
pub enum Parity {
    NoParity,
    Odd,
    Even,
    ForcedParity1,
    ForcedParity0,
}

#[allow(missing_docs)]
pub struct UartConfig {
    baud: u32,
    word_length: u8,
    parity: Parity,
    stop_bit: u8,
}

impl UartConfig {
    /// Holds parameters for each individual UART
    ///
    /// # Example
    /// let config UartConfig::new(9600, 8, Parity::None, 1);
    pub fn new(baud: u32, word_length: u8, parity: Parity, stop_bit: u8) -> Self {
        Self {
            // Maximum baudrate is 115200
            baud,
            // 5, 6, 7 or 8 bits
            word_length,
            // Use Parity enum
            parity,
            // Use 0, 1 or 2 (insert table here)
            stop_bit,
        }
    }
    pub fn baudrate(mut self, baud: u32) -> Self {
        self.baud = baud;
        self
    }
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            baud: 115200,
            word_length: 8,
            parity: Parity::NoParity,
            stop_bit: 1,
        }
    }
}

pub trait Bus {
    type Error;

    fn write_register(
        &mut self,
        channel: Channel,
        reg: Registers,
        payload: u8,
    ) -> Result<(), Self::Error>;

    fn read_register(&mut self, channel: Channel, reg: Registers) -> Result<u8, Self::Error>;
}

#[derive(Debug)]
pub struct SC16IS752i2c<I2C> {
    address: u8,
    i2c: I2C,
}

impl<I2C> SC16IS752i2c<I2C>
where
    I2C: I2c,
{
    pub fn new(device_address: u8, i2c: I2C) -> Self {
        let mut address = device_address;
        if !(0x48..=0x57).contains(&device_address) {
            address = device_address >> 1
        }
        Self { address, i2c }
    }
}

impl<I2C> Bus for SC16IS752i2c<I2C>
where
    I2C: I2c,
{
    type Error = I2C::Error;
    fn read_register(&mut self, channel: Channel, reg: Registers) -> Result<u8, Self::Error> {
        let mut result = [0];
        self.i2c
            .write_read(
                self.address,
                &[(reg as u8) << 3 | (channel as u8) << 1],
                &mut result,
            )
            .and(Ok(result[0]))
    }

    fn write_register(
        &mut self,
        channel: Channel,
        reg: Registers,
        payload: u8,
    ) -> Result<(), Self::Error> {
        self.i2c.write(
            self.address,
            &[(reg as u8) << 3 | (channel as u8) << 1u8, payload],
        )
    }
}

#[derive(Debug)]
pub struct SC16IS752spi<SPI> {
    spi: SPI,
}

impl<SPI> SC16IS752spi<SPI>
where
    SPI: SpiDevice,
{
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }
}

impl<SPI> Bus for SC16IS752spi<SPI>
where
    SPI: SpiDevice,
{
    type Error = SPI::Error;
    fn read_register(&mut self, channel: Channel, reg: Registers) -> Result<u8, Self::Error> {
        let mut result = [0u8; 2];
        self.spi
            .transfer(
                &mut result,
                &[1 << 7 | (reg as u8) << 3 | (channel as u8) << 1, 0xaa],
            )
            .and(Ok(result[1]))
    }

    fn write_register(
        &mut self,
        channel: Channel,
        reg: Registers,
        payload: u8,
    ) -> Result<(), Self::Error> {
        self.spi
            .write(&[(reg as u8) << 3 | (channel as u8) << 1u8, payload])
    }
}

#[derive(Debug)]
pub struct SC16IS752<BUS>
where
    BUS: Bus,
{
    bus: BUS,
    xtal_freq: u32,
    fifo: [u8; 2],
    peek_flags: [bool; 2],
    peek_buf: [Option<u8>; 2],
}

impl<BUS> SC16IS752<BUS>
where
    BUS: Bus,
{
    pub fn new(bus: BUS, xtal_freq: u32) -> Self {
        Self {
            bus,
            xtal_freq,
            fifo: [0u8; 2],
            peek_flags: [false; 2],
            peek_buf: [None; 2],
        }
    }

    /// Initalises a single UART using UartConfig struct
    pub fn initialise_uart(
        &mut self,
        channel: Channel,
        config: UartConfig,
    ) -> Result<(), BUS::Error> {
        self.fifo_enable(channel, true)?;
        self.set_baudrate(channel, config.baud)?;
        self.set_line(channel, config.word_length, config.parity, config.stop_bit)?;
        Ok(())
    }

    fn read_register(&mut self, channel: Channel, reg: Registers) -> Result<u8, BUS::Error> {
        self.bus.read_register(channel, reg)
    }

    fn write_register(
        &mut self,
        channel: Channel,
        reg: Registers,
        payload: u8,
    ) -> Result<(), BUS::Error> {
        self.bus.write_register(channel, reg, payload)
    }

    fn set_baudrate(&mut self, channel: Channel, baudrate: u32) -> Result<(), BUS::Error> {
        let prescaler = match self.read_register(channel, Registers::MCR)? {
            0 => 1,
            _ => 4,
        };
        let divisor = (self.xtal_freq / prescaler as u32) / (baudrate * 16);

        let mut temp_line_control_register = self.read_register(channel, Registers::LCR)?;

        // Move to special register mode where RhrThr is DLL and IER is DLH registers.
        temp_line_control_register |= 0x80;
        self.write_register(channel, Registers::LCR, temp_line_control_register)?;

        self.write_register(channel, Registers::RhrThr, divisor.try_into().unwrap())?;
        self.write_register(channel, Registers::IER, (divisor >> 8).try_into().unwrap())?;

        // Move out of special register mode
        temp_line_control_register &= 0x7F;
        self.write_register(channel, Registers::LCR, temp_line_control_register)?;

        // {
        //     let actual_baudrate = (CRYSTAL_FREQ / prescaler as u32) / (16 * divisor);
        //     let error = (actual_baudrate - baudrate) * 1000 / baudrate;

        //     println!("UART {channel}: Desired baudrate: {baudrate}");
        //     println!("UART {channel}: Calculated divisor: {divisor}");
        //     println!("UART {channel}: Actual baudrate: {actual_baudrate}");
        //     println!("UART {channel}: Baudrate error: {error}");
        // }
        Ok(())
    }

    fn set_line(
        &mut self,
        channel: Channel,
        data_length: u8,
        parity_select: Parity,
        stop_length: u8,
    ) -> Result<(), BUS::Error> {
        let mut temp_line_control_register: u8 = self.read_register(channel, Registers::LCR)?;
        temp_line_control_register &= 0xC0;
        {
            //println!("line_control_register Register: {temp_line_control_register:#04x}");
        }

        match data_length {
            5 => {}
            6 => temp_line_control_register |= 0x01,
            7 => temp_line_control_register |= 0x02,
            _ => temp_line_control_register |= 0x03,
        };
        if stop_length == 2 {
            temp_line_control_register |= 0x04;
        }
        match parity_select {
            Parity::NoParity => {}
            Parity::Odd => temp_line_control_register |= 0x08,
            Parity::Even => temp_line_control_register |= 0x18,
            Parity::ForcedParity1 => temp_line_control_register |= 0x28,
            Parity::ForcedParity0 => temp_line_control_register |= 0x38,
        }
        self.write_register(channel, Registers::LCR, temp_line_control_register)
    }

    /// This register is used to set an I/O pin direction. Bit 0 to bit 7 controls GPIO0 to GPIO7.
    pub fn gpio_set_pin_mode(
        &mut self,
        pin_number: GPIO,
        pin_direction: PinMode,
    ) -> Result<(), BUS::Error> {
        let mut temp_io_direction_register = self.read_register(Channel::A, Registers::IODir)?;
        match pin_direction {
            PinMode::Output => temp_io_direction_register |= 0x01 << (pin_number as u8),
            PinMode::Input => temp_io_direction_register &= !(0x01 << (pin_number as u8)),
        }
        self.write_register(Channel::A, Registers::IODir, temp_io_direction_register)
    }

    pub fn gpio_set_pin_state(
        &mut self,
        pin_number: GPIO,
        pin_state: PinState,
    ) -> Result<(), BUS::Error> {
        let mut temp_io_direction_register = self.read_register(Channel::A, Registers::IOState)?;
        match pin_state {
            PinState::High => temp_io_direction_register |= 0x01 << (pin_number as u8),
            PinState::Low => temp_io_direction_register &= !(0x01 << (pin_number as u8)),
        }
        self.write_register(Channel::A, Registers::IOState, temp_io_direction_register)
    }

    pub fn gpio_get_pin_state(&mut self, pin_number: GPIO) -> Result<PinState, BUS::Error> {
        let temp_iostate = self.read_register(Channel::A, Registers::IOState)?;

        if (temp_iostate & (0x01 << (pin_number as u8))) == 0 {
            return Ok(PinState::Low);
        }
        Ok(PinState::High)
    }

    pub fn gpio_get_port_state(&mut self) -> Result<u8, BUS::Error> {
        self.read_register(Channel::A, Registers::IOState)
    }

    pub fn gpio_set_port_mode(&mut self, port_io: u8) -> Result<(), BUS::Error> {
        self.write_register(Channel::A, Registers::IODir, port_io)
    }

    pub fn gpio_set_port_state(&mut self, port_state: u8) -> Result<(), BUS::Error> {
        self.write_register(Channel::A, Registers::IOState, port_state)
    }

    pub fn set_pin_interrupt(
        &mut self,
        io_interrupt_enable_register: u8,
    ) -> Result<(), BUS::Error> {
        self.write_register(
            Channel::A,
            Registers::IOIntEna,
            io_interrupt_enable_register,
        )
    }

    pub fn reset_device(&mut self) -> Result<(), BUS::Error> {
        let mut reg: u8 = self.read_register(Channel::A, Registers::IOControl)?;
        reg |= 0x08;
        self.write_register(Channel::A, Registers::IOControl, reg)
    }

    pub fn modem_pin(&mut self, state: bool) -> Result<(), BUS::Error> {
        let mut temp_io_control_register = self.read_register(Channel::A, Registers::IOControl)?;

        if state {
            temp_io_control_register |= 0x02;
        } else {
            temp_io_control_register &= 0xFD;
        }

        self.write_register(Channel::A, Registers::IOControl, temp_io_control_register)
    }

    pub fn gpio_latch(&mut self, latch: bool) -> Result<(), BUS::Error> {
        let mut temp_io_control_register = self.read_register(Channel::A, Registers::IOControl)?;

        if !latch {
            temp_io_control_register &= 0xFE;
        } else {
            temp_io_control_register |= 0x01;
        }

        self.write_register(Channel::A, Registers::IOControl, temp_io_control_register)
    }

    pub fn interrupt_control(
        &mut self,
        channel: Channel,
        interrupt_enable_register: u8,
    ) -> Result<(), BUS::Error> {
        self.write_register(channel, Registers::IER, interrupt_enable_register)
    }

    pub fn interrupt_pending_test(&mut self, channel: Channel) -> Result<u8, BUS::Error> {
        let ipt = self.read_register(channel, Registers::FcrIir)?;
        Ok(ipt & 0x01)
    }

    pub fn isr(&mut self, channel: Channel) -> Result<InterruptEventTest, BUS::Error> {
        let mut interrupt_identification_register =
            self.read_register(channel, Registers::FcrIir)?;
        // interrupt_identification_register >>= 1;
        interrupt_identification_register &= 0x3E;
        match interrupt_identification_register {
            0x06 => Ok(InterruptEventTest::RECEIVE_LINE_STATUS_ERROR),
            0x0C => Ok(InterruptEventTest::RECEIVE_TIMEOUT_INTERRUPT),
            0x04 => Ok(InterruptEventTest::RHR_INTERRUPT),
            0x02 => Ok(InterruptEventTest::THR_INTERRUPT),
            0x00 => Ok(InterruptEventTest::MODEM_INTERRUPT),
            0x30 => Ok(InterruptEventTest::INPUT_PIN_CHANGE_STATE),
            0x10 => Ok(InterruptEventTest::RECEIVE_XOFF),
            0x20 => Ok(InterruptEventTest::CTS_RTS_CHANGE),
            _ => Ok(InterruptEventTest::UNKNOWN),
        }
    }

    pub fn fifo_enable(&mut self, channel: Channel, state: bool) -> Result<(), BUS::Error> {
        let mut fifo_control_register = self.read_register(channel, Registers::FcrIir)?;

        if !state {
            fifo_control_register &= 0xFE;
        } else {
            fifo_control_register |= 0x01;
        }
        self.write_register(channel, Registers::FcrIir, fifo_control_register)
    }

    pub fn fifo_reset(&mut self, channel: Channel, state: bool) -> Result<(), BUS::Error> {
        let mut temp_fcr = self.read_register(channel, Registers::FcrIir)?;

        if !state {
            temp_fcr |= 0x04;
        } else {
            temp_fcr |= 0x02;
        }
        self.write_register(channel, Registers::FcrIir, temp_fcr)
    }

    pub fn fifo_set_trigger_level(
        &mut self,
        channel: Channel,
        rx_fifo: bool,
        length: u8,
    ) -> Result<(), BUS::Error> {
        let mut temp_reg = self.read_register(channel, Registers::MCR)?;
        temp_reg |= 0x04;
        self.write_register(channel, Registers::MCR, temp_reg)?; // SET MCR[2] to '1' to use TLR register or trigger level control in FCR
        // register

        temp_reg = self.read_register(channel, Registers::FcrIir)?;
        self.write_register(channel, Registers::FcrIir, temp_reg | 0x10)?; // set ERF[4] to '1' to use the  enhanced features

        if !rx_fifo {
            self.write_register(channel, Registers::SprTlr, length << 4)?; // Tx FIFO trigger level setting
        } else {
            self.write_register(channel, Registers::SprTlr, length)?; // Rx FIFO Trigger level setting
        }
        self.write_register(channel, Registers::FcrIir, temp_reg) // restore EFR register
    }

    pub fn fifo_available_data(&mut self, channel: Channel) -> Result<u8, BUS::Error> {
        // if self.fifo[channel as usize] == 0 {
        self.fifo[channel as usize] = self.read_register(channel, Registers::RXLVL)?;
        // }
        Ok(self.fifo[channel as usize])
    }

    pub fn fifo_available_space(&mut self, channel: Channel) -> Result<u8, BUS::Error> {
        self.read_register(channel, Registers::TXLVL)
    }

    fn write_byte(&mut self, channel: Channel, val: &u8) -> Result<(), BUS::Error> {
        self.write_register(channel, Registers::RhrThr, *val)
    }

    pub fn write(&mut self, channel: Channel, payload: &[u8]) -> Result<usize, BUS::Error> {
        let space_left = self.fifo_available_space(channel)? as usize;
        let len = payload.len().min(space_left);

        for i in 0..len {
            self.write_byte(channel, &payload[i])?;
        }
        Ok(len)
    }

    fn read_byte(&mut self, channel: Channel) -> Result<Option<u8>, BUS::Error> {
        if self.fifo_available_data(channel)? == 0 {
            //println!("No data");
            return Ok(None);
        }
        Ok(Some(self.read_register(channel, Registers::RhrThr)?))
    }

    pub fn read(&mut self, channel: Channel, buf: &mut [u8]) -> Result<usize, BUS::Error> {
        let available = self.fifo_available_data(channel)? as usize;
        let len = buf.len().min(available);

        for i in 0..len {
            if let Ok(Some(byte)) = self.read_byte(channel) {
                buf[i] = byte;
            }
        }
        Ok(len)
    }

    pub fn enable_features(
        &mut self,
        channel: Channel,
        feature: FeaturesRegister,
        enable: bool,
    ) -> Result<(), BUS::Error> {
        let mut temp_extra_features_control_register =
            self.read_register(channel, Registers::EFCR)?;

        if !enable {
            temp_extra_features_control_register |= feature as u8;
        } else {
            temp_extra_features_control_register &= !(feature as u8);
        }
        self.write_register(
            channel,
            Registers::EFCR,
            temp_extra_features_control_register,
        )
    }

    pub fn ping(&mut self) -> Result<bool, BUS::Error> {
        self.write_register(Channel::A, Registers::SprTlr, 0x55)?;

        if self.read_register(Channel::A, Registers::SprTlr)? != 0x55 {
            return Ok(false);
        }

        self.write_register(Channel::A, Registers::SprTlr, 0xAA)?;

        if self.read_register(Channel::A, Registers::SprTlr)? != 0xAA {
            return Ok(false);
        }

        self.write_register(Channel::B, Registers::SprTlr, 0x55)?;

        if self.read_register(Channel::B, Registers::SprTlr)? != 0x55 {
            return Ok(false);
        }

        self.write_register(Channel::B, Registers::SprTlr, 0xAA)?;

        if self.read_register(Channel::B, Registers::SprTlr)? != 0xAA {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn flush(&mut self, channel: Channel) -> Result<(), BUS::Error> {
        let mut tmp_line_status_register: u8 = 0;

        while (tmp_line_status_register & 0x20) == 0 {
            tmp_line_status_register = self.read_register(channel, Registers::LSR)?;
        }
        Ok(())
    }

    pub fn peek(&mut self, channel: Channel) -> Result<(), BUS::Error> {
        if self.peek_flags[channel as usize] {
            self.peek_buf[channel as usize] = self.read_byte(channel)?;

            if self.peek_buf[channel as usize].is_some() {
                self.peek_flags[channel as usize] = true;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
