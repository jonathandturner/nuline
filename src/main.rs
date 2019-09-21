// #![feature(generators)]
// #![feature(proc_macro_hygiene)]
// use futures::stream::BoxStream;
// use futures::stream::StreamExt;
// use futures::{FutureExt, Stream};
// use futures_async_stream::async_stream_block;
// //use async_std::{io, net::TcpStream, prelude::*, task};
// use std::mem;

use libc::{c_int, termios as Termios};
use std::io;
use std::io::{Read, Write};
use std::mem;

fn unwrap(t: i32) -> io::Result<()> {
    if t == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

struct Terminal {
    raw_mode_enabled: bool,
    original_terminal_mode: Option<Termios>,
}

impl Terminal {
    pub fn new() -> Terminal {
        Terminal {
            raw_mode_enabled: false,
            original_terminal_mode: None,
        }
    }

    /// Transform the given mode into an raw mode (non-canonical) mode.
    pub fn raw_terminal_attr(termios: &mut Termios) {
        extern "C" {
            pub fn cfmakeraw(termptr: *mut Termios);
        }
        unsafe { cfmakeraw(termios) }
    }

    pub fn get_terminal_attr() -> io::Result<Termios> {
        extern "C" {
            pub fn tcgetattr(fd: c_int, termptr: *mut Termios) -> c_int;
        }
        unsafe {
            let mut termios = mem::zeroed();
            unwrap(tcgetattr(0, &mut termios))?;
            Ok(termios)
        }
    }

    pub fn set_terminal_attr(termios: &Termios) -> io::Result<()> {
        extern "C" {
            pub fn tcsetattr(fd: c_int, opt: c_int, termptr: *const Termios) -> c_int;
        }
        unwrap(unsafe { tcsetattr(0, 0, termios) }).and(Ok(()))
    }

    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
        let mut ios = Terminal::get_terminal_attr()?;
        let prev_ios = ios;

        if self.original_terminal_mode.is_none() {
            self.original_terminal_mode = Some(prev_ios.clone());
        }

        self.raw_mode_enabled = true;

        Terminal::raw_terminal_attr(&mut ios);
        Terminal::set_terminal_attr(&ios)?;
        Ok(())
    }

    pub fn disable_raw_mode(&mut self) -> io::Result<()> {
        if let Some(ref mode) = self.original_terminal_mode {
            Terminal::set_terminal_attr(mode)?;

            self.raw_mode_enabled = false;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum KeyEvent {
    Line(String),
    Exit,
}

// pub struct OutputStream {
//     pub(crate) values: BoxStream<'static, KeyEvent>,
// }

// pub trait ToOutputStream {
//     fn to_output_stream(self) -> OutputStream;
// }

// impl<T, U> ToOutputStream for T
// where
//     T: Stream<Item = U> + Send + 'static,
//     U: Into<KeyEvent>,
// {
//     fn to_output_stream(self) -> OutputStream {
//         OutputStream {
//             values: self.map(|item| item.into()).boxed(),
//         }
//     }
// }

// fn build_event_stream() -> OutputStream {
//     let stream = async_stream_block! {
//         loop {
//             println!("Input >");
//             let mut buffer = String::new();
//             let stdin = io::stdin();
//             let input = stdin.read_line(&mut buffer).await;

//             println!("Done reading");

//             if input.is_ok() {
//                 if buffer.trim() == "exit" {
//                     yield KeyEvent::Exit;
//                 } else {
//                     yield KeyEvent::Line(buffer.clone());
//                 }
//             } else {
//                 break;
//             }
//         }
//     };

//     stream.to_output_stream()
// }

// impl From<BoxStream<'static, KeyEvent>> for OutputStream {
//     fn from(input: BoxStream<'static, KeyEvent>) -> OutputStream {
//         OutputStream { values: input }
//     }
// }

// impl Stream for OutputStream {
//     type Item = KeyEvent;

//     fn poll_next(
//         mut self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> core::task::Poll<Option<Self::Item>> {
//         Stream::poll_next(std::pin::Pin::new(&mut self.values), cx)
//     }
// }

// async fn term_loop() {
//     let mut term = Terminal::new();

//     let _ = term.enable_raw_mode();

//     loop {
//         let mut events = build_event_stream();

//         let events = events.collect::<Vec<_>>().await;
//         for event in events  {
//             println!("{:?}", event);
//         }
//     }

//     let _ = term.disable_raw_mode();
// }

fn read_char() -> io::Result<u8> {
    let mut buffer = vec![0; 1];
    let mut stdin = io::stdin();
    stdin.read_exact(&mut buffer)?;

    Ok(buffer[0])
}

fn read_position() -> io::Result<(u16, u16)> {
    let mut buffer = vec![];

    println!("\x1b[6n");
    let mut input = read_char()?;

    while input != 82 {
        buffer.push(input);
        input = read_char()?;
    }

    let buffer = &buffer[2..];

    let positions_raw: Vec<_> = buffer.split(|x| *x == 59).collect();
    let mut positions = vec![];

    for r in positions_raw {
        let mut current_val = 0u16;
        for digit in r {
            current_val = current_val * 10 + (*digit as u16 - ('0' as u16));
        }
        positions.push(current_val);
    }

    Ok((positions[0], positions[1]))
}

fn goto_position(row: u16, column: u16) -> io::Result<()> {
    println!("\x1b[{};{}f", row, column);
    Ok(())
}

fn term_loop() {
    let mut term = Terminal::new();
    let _ = term.enable_raw_mode();

    print!("> ");
    let _ = std::io::stdout().flush();
    let mut buffer = String::new();
    loop {
        if let Ok(c) = read_char() {
            if c == 13 {
                // Carriage return
                if buffer == "quit" {
                    break;
                } else if buffer == "jump" {
                    let _ = goto_position(10, 10);
                } else if buffer == "where" {
                    if let Ok((height, width)) = read_position() {
                        print!("{}, {}\r\n", height, width);
                    }
                }
                buffer.clear();
                print!("> ");
                let _ = std::io::stdout().flush();
            } else {
                print!("{}", c as char);
                let _ = std::io::stdout().flush();
                buffer.push(c as char);
            }
        }
    }

    let _ = term.disable_raw_mode();
}

fn main() {
    //futures::executor::block_on(term_loop());
    term_loop();
}
