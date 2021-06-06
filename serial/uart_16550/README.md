# uart_16550

[![Build Status](https://github.com/rust-osdev/uart_16550/workflows/Build/badge.svg)](https://github.com/rust-osdev/uart_16550/actions?query=workflow%3ABuild) [![Docs.rs Badge](https://docs.rs/uart_16550/badge.svg)](https://docs.rs/uart_16550/)

Minimal support for uart_16550 serial and memory mapped I/O.

## Usage

### With `port_{stable, nightly}` feature

```rust
use uart_16550::SerialPort;

const SERIAL_IO_PORT: u16 = 0x3F8;

let mut serial_port = unsafe { SerialPort::new(SERIAL_IO_PORT) };
serial_port.init();

// Now the serial port is ready to be used. To send a byte:
serial_port.send(42);

// To receive a byte:
let data = serial_port.receive();
```

### With `mmio_{stable, nightly}` feature

```rust
use uart_16550::MmioSerialPort;

const SERIAL_PORT_BASE_ADDRESS: usize = 0x1000_0000;

let mut serial_port = unsafe { MmioSerialPort::new(SERIAL_PORT_BASE_ADDRESS) };
serial_port.init();

// Now the serial port is ready to be used. To send a byte:
serial_port.send(42);

// To receive a byte:
let data = serial_port.receive();
```

## License

Licensed under the MIT license ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>).

## Crate Feature Flags

* `port_nightly`: This is the default.
* `port_stable`: Use this to build with non-nightly rust. Needs `default-features = false`.
* `mmio_nightly`: Use this to initialize serial port through memory mapped I/O.
* `mmio_stable`: Use this to build with non-nightly rust. Needs `default-features = false`.

## Building with stable rust

This needs to have the [compile-time requirements](https://github.com/alexcrichton/cc-rs#compile-time-requirements) of the `cc` crate installed on your system.
It was currently only tested on Linux and MacOS.
