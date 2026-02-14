use std::{
    error::Error,
    io::{Write, stdin, stdout},
    process::exit,
};

use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::ToMainScreen};

use crate::{device::DeviceMessage, host_device::HostDevice, native::NativeHostSender};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Shell {
    uart_tx: NativeHostSender,
}

impl HostDevice for Shell {
    fn run(self: Box<Self>) {
        Shell::run(*self).unwrap();
    }
}

fn make_message(c: char) -> DeviceMessage {
    DeviceMessage::Uart(c)
}

impl Shell {
    pub fn new(uart_tx: NativeHostSender) -> Self {
        Self { uart_tx }
    }

    pub fn run(self) -> Result<()> {
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode()?;

        for k in stdin.keys() {
            let k = k?;

            match k {
                Key::Char(c) => {
                    self.uart_tx.send(make_message(c))?;
                }
                Key::Backspace => self.uart_tx.send(make_message('\x08'))?,
                Key::Ctrl('d') => {
                    write!(stdout, "{}", ToMainScreen)?;
                    drop(stdout);
                    exit(0);
                }
                Key::Ctrl('a') => self.uart_tx.send(make_message('\x01'))?,
                Key::Ctrl('c') => self.uart_tx.send(make_message('\x03'))?,
                Key::Ctrl('e') => self.uart_tx.send(make_message('\x05'))?,
                Key::Ctrl('h') => self.uart_tx.send(make_message('\x08'))?,
                Key::Ctrl('l') => self.uart_tx.send(make_message('\x0c'))?,
                Key::Ctrl('n') => self.uart_tx.send(make_message('\x0e'))?,
                Key::Ctrl('p') => self.uart_tx.send(make_message('\x10'))?,
                Key::Ctrl('u') => self.uart_tx.send(make_message('\x15'))?,
                Key::Ctrl('w') => self.uart_tx.send(make_message('\x18'))?,
                Key::Ctrl('[') => self.uart_tx.send(make_message('\x1b'))?,
                _ => {}
            }
        }

        Ok(())
    }
}
