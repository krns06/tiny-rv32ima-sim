use std::{
    io::{Write, stdin, stdout},
    process::exit,
    sync::mpsc::Sender,
};

use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::ToMainScreen};

pub fn run_shell(tx: Sender<char>) {
    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode().unwrap();

    for k in stdin.keys() {
        match k.as_ref().unwrap() {
            Key::Char(c) => {
                tx.send(*c).unwrap();
            }
            Key::Backspace => tx.send('\x08').unwrap(),
            Key::Ctrl('d') => {
                write!(stdout, "{}", ToMainScreen).unwrap();
                drop(stdout);
                exit(0);
            }
            Key::Ctrl('c') => tx.send('\x03').unwrap(),
            Key::Ctrl('h') => tx.send('\x08').unwrap(),
            Key::Ctrl('l') => tx.send('\x0c').unwrap(),
            Key::Ctrl('n') => tx.send('\x0e').unwrap(),
            Key::Ctrl('p') => tx.send('\x10').unwrap(),
            Key::Ctrl('u') => tx.send('\x15').unwrap(),
            _ => {}
        }
    }
}
