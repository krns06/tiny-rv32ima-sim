use std::{
    error::Error,
    io::{Write, stdin, stdout},
    process::exit,
    sync::mpsc::Sender,
};

use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::ToMainScreen};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn run_shell(tx: Sender<char>) -> Result<()> {
    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode()?;

    for k in stdin.keys() {
        let k = k?;

        match k {
            Key::Char(c) => {
                tx.send(c)?;
            }
            Key::Backspace => tx.send('\x08')?,
            Key::Ctrl('d') => {
                write!(stdout, "{}", ToMainScreen)?;
                drop(stdout);
                exit(0);
            }
            Key::Ctrl('a') => tx.send('\x01')?,
            Key::Ctrl('c') => tx.send('\x03')?,
            Key::Ctrl('e') => tx.send('\x05')?,
            Key::Ctrl('h') => tx.send('\x08')?,
            Key::Ctrl('l') => tx.send('\x0c')?,
            Key::Ctrl('n') => tx.send('\x0e')?,
            Key::Ctrl('p') => tx.send('\x10')?,
            Key::Ctrl('u') => tx.send('\x15')?,
            Key::Ctrl('w') => tx.send('\x18')?,
            Key::Ctrl('[') => tx.send('\x1b')?,
            _ => {}
        }
    }

    Ok(())
}
