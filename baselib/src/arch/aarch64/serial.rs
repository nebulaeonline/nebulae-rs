// Adapted from phil-opp & Mike Krinkin & uart-16550
#![cfg(target_arch = "aarch64")]

use core::ptr;
use core::fmt::Write;

use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref SERIAL1: Mutex<PL011> = {
        let serial_port = PL011::new(
            /* base_address = */0x9000000,
            /* base_clock = */24000000);
        Mutex::new(serial_port)
    };
}

#[derive(Copy, Clone)]
pub struct PL011 {
    base_address: u64,
    base_clock: u32,
    baudrate: u32,
    data_bits: u32,
    stop_bits: u32,
}

#[derive(Copy, Clone)]
enum Register {
    UARTDR = 0x000,
    UARTFR = 0x018,
    UARTIBRD = 0x024,
    UARTFBRD = 0x028,
    UARTLCR = 0x02c,
    UARTCR = 0x030,
    UARTIMSC = 0x038,
    UARTICR = 0x044,
    UARTDMACR = 0x048,
}

const UARTFR_BUSY: u32 = 1 << 3;
const UARTLCR_STP2: u32 = 1 << 3;
const UARTLCR_FEN: u32 = 1 << 4;
const UARTCR_EN: u32 = 1 << 0;
const UARTCR_TXEN: u32 = 1 << 8;


impl PL011 {
    pub fn new(base_address: u64, base_clock: u32) -> Self {
        let dev = PL011 {
            base_address,
            base_clock,
            baudrate: 115200,
            data_bits: 8,
            stop_bits: 1,
        };

        dev.wait_tx_complete();
        dev.reset();

        dev
    }

    pub fn reset(&self) {
        let cr = self.load(Register::UARTCR);
        let lcr = self.load(Register::UARTLCR);

        self.store(Register::UARTCR, cr & !UARTCR_EN);
        self.wait_tx_complete();
        self.store(Register::UARTLCR, lcr & !UARTLCR_FEN);
        self.store(Register::UARTIMSC, 0x7ff);
        self.store(Register::UARTICR, 0x7ff);
        self.store(Register::UARTDMACR, 0x0);

        let (ibrd, fbrd) = self.calculate_divisors();

        self.store(Register::UARTIBRD, ibrd);
        self.store(Register::UARTFBRD, fbrd);

        let mut lcr = ((self.data_bits - 1) & 0x3) << 5;
        if self.stop_bits == 2 {
            lcr |= UARTLCR_STP2;
        }
        self.store(Register::UARTLCR, lcr);

        self.store(Register::UARTCR, UARTCR_TXEN);
        self.store(Register::UARTCR, UARTCR_TXEN | UARTCR_EN);
    }

    pub fn send(&self, data: &str) {
        self.wait_tx_complete();

        for b in data.bytes() {
            if b == b'\n' {
                self.store(Register::UARTDR, b'\r' as u32);
                self.wait_tx_complete();
            }
            self.store(Register::UARTDR, b as u32);
            self.wait_tx_complete();
        }
    }

    fn wait_tx_complete(&self) {
        loop {
            if (self.load(Register::UARTFR) & UARTFR_BUSY) == 0 {
                return;
            }
        }
    }

    fn load(&self, r: Register) -> u32 {
        let addr = self.base_address + (r as u64);

        unsafe { ptr::read_volatile(addr as *const u32) }
    }

    fn store(&self, r: Register, value: u32) {
        let addr = self.base_address + (r as u64);

        unsafe { ptr::write_volatile(addr as *mut u32, value); }
    }

    fn calculate_divisors(&self) -> (u32, u32) {
        let div =
            (8 * self.base_clock + self.baudrate) / (2 * self.baudrate);

        ((div >> 6) & 0xffff, div & 0x3f)
    }
}

impl Write for PL011 {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        self.send(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {    
    SERIAL1
        .lock()
        .write_fmt(args)
        .expect("Printing to serial failed");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::_print(format_args!($($arg)*))
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
