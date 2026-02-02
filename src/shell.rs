use std::{
    io::{Write, stdin, stdout},
    process::exit,
    sync::mpsc::Sender,
};

use termion::{
    event::Key,
    input::TermRead,
    raw::IntoRawMode,
    screen::{IntoAlternateScreen, ToMainScreen},
};

pub fn run_shell(tx: Sender<char>) {
    let stdin = stdin();
    let mut stdout = stdout()
        .into_raw_mode()
        .unwrap()
        .into_alternate_screen()
        .unwrap();

    for k in stdin.keys() {
        match k.as_ref().unwrap() {
            Key::Char(c) => {
                tx.send(*c).unwrap();
            }
            Key::Ctrl('d') => {
                write!(stdout, "{}", ToMainScreen).unwrap();
                exit(0);
            }
            Key::Ctrl('c') => tx.send('\x03').unwrap(),
            Key::Ctrl('l') => tx.send('\x0c').unwrap(),
            Key::Ctrl('u') => tx.send('\x15').unwrap(),
            _ => {}
        }
    }
}
