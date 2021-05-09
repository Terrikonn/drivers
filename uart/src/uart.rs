//! MMIO driver for [16550 UART](https://en.wikipedia.org/wiki/16550_UART)

use core::{convert::TryInto, fmt};

use bit_field::BitField;

pub struct NoDataError;

pub struct Uart {
    base_address: usize,
}

impl fmt::Write for Uart {
    fn write_str(&mut self, out: &str) -> Result<(), fmt::Error> {
        for byte in out.bytes() {
            self.send(byte);
        }

        Ok(())
    }
}

impl Uart {
    pub const fn new(base_address: usize) -> Self {
        Self { base_address }
    }

    pub fn init(&mut self) {
        let ptr = self.base_address as *mut u8;
        unsafe {
            let line_control_register: u8 = (1 << 0) | (1 << 1);
            self.set_word_length(line_control_register);
            self.enable_fifo();
            self.enable_receiver_buffer_interrupts();

            // If we cared about the divisor, the code below would
            // set the divisor from a global clock rate of 22.729
            // MHz (22,729,000 cycles per second) to a signaling
            // rate of 2400 (BAUD). We usually have much faster
            // signalling rates nowadays, but this demonstrates what
            // the divisor actually does. The formula given in the
            // NS16500A specification for calculating the divisor
            // is:
            // divisor = ceil( (clock_hz) / (baud_sps x 16) )
            // So, we substitute our values and get:
            // divisor = ceil( 22_729_000 / (2400 x 16) )
            // divisor = ceil( 22_729_000 / 38_400 )
            // divisor = ceil( 591.901 ) = 592

            // The divisor register is two bytes (16 bits), so we
            // need to split the value 592 into two bytes.
            // Typically, we would calculate this based on measuring
            // the clock rate, but again, for our purposes [qemu],
            // this doesn't really do anything.
            let divisor: u16 = 592;
            let divisor_least: u8 = divisor.get_bits(0..8).try_into().unwrap();
            let divisor_most: u8 = divisor.get_bits(8..16).try_into().unwrap();
            // let divisor_least: u8 = (divisor & 0xff).try_into().unwrap();
            // let divisor_most: u8 = (divisor >> 8).try_into().unwrap();

            // Notice that the divisor register DLL (divisor latch
            // least) and DLM (divisor latch most) have the same
            // base address as the receiver/transmitter and the
            // interrupt enable register. To change what the base
            // address points to, we open the "divisor latch" by
            // writing 1 into the Divisor Latch Access Bit (DLAB),
            // which is bit index 7 of the Line Control Register
            // (LCR) which is at base_address + 3.
            ptr.add(3).write_volatile(line_control_register | 1 << 7);

            // Now, base addresses 0 and 1 point to DLL and DLM,
            // respectively. Put the lower 8 bits of the divisor
            // into DLL
            ptr.add(0).write_volatile(divisor_least);
            ptr.add(1).write_volatile(divisor_most);

            // Now that we've written the divisor, we never have to
            // touch this again. In hardware, this will divide the
            // global clock (22.729 MHz) into one suitable for 2,400
            // signals per second. So, to once again get access to
            // the RBR/THR/IER registers, we need to close the DLAB
            // bit by clearing it to 0.
            ptr.add(3).write_volatile(line_control_register);
        }
    }

    unsafe fn set_word_length(&mut self, line_control_register: u8) {
        let ptr = self.base_address as *mut u8;
        // First, set the word length, which
        // are bits 0 and 1 of the line control register (LCR)
        // which is at base_address + 3
        // We can easily write the value 3 here or 0b11, but I'm
        // extending it so that it is clear we're setting two
        // individual fields
        //             Word 0     Word 1
        //             ~~~~~~     ~~~~~~
        ptr.add(3).write_volatile(line_control_register);
    }

    unsafe fn enable_fifo(&mut self) {
        let ptr = self.base_address as *mut u8;
        // Now, enable the FIFO, which is bit index 0 of the
        // FIFO control register (FCR at offset 2).
        // Again, we can just write 1 here, but when we use left
        // shift, it's easier to see that we're trying to write
        // bit index #0.
        ptr.add(2).write_volatile(1 << 0);
    }

    unsafe fn enable_receiver_buffer_interrupts(&mut self) {
        let ptr = self.base_address as *mut u8;
        // Enable receiver buffer interrupts, which is at bit
        // index 0 of the interrupt enable register (IER at
        // offset 1).
        ptr.add(1).write_volatile(1 << 0);
    }

    pub fn send(&mut self, byte: u8) {
        let ptr = self.base_address as *mut u8;
        unsafe {
            ptr.add(0).write_volatile(byte);
        }
    }

    pub fn receive(&mut self) -> Option<u8> {
        let ptr = self.base_address as *mut u8;
        unsafe {
            if ptr.add(5).read_volatile() & 1 == 0 {
                // The DR bit is 0, meaning no data
                None
            } else {
                // The DR bit is 1, meaning data!
                Some(ptr.add(0).read_volatile())
            }
        }
    }
}
