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

fn read_char() -> io::Result<u8> {
    let mut buffer = vec![0; 1];
    let mut stdin = io::stdin();
    stdin.read_exact(&mut buffer)?;

    Ok(buffer[0])
}

fn read_position() -> io::Result<(u16, u16)> {
    let mut buffer = vec![];

    print!("\x1b[6n");
    let _ = std::io::stdout().flush();
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
    print!("\x1b[{};{}f", row, column);
    let _ = std::io::stdout().flush();
    Ok(())
}

fn paint_string(s: &str, row: u16, col: u16) -> io::Result<()> {
    goto_position(row, col)?;

    print!("{}", s);
    let _ = std::io::stdout().flush();
    Ok(())
}

fn term_loop() -> io::Result<()> {
    let mut term = Terminal::new();
    let _ = term.enable_raw_mode();

    print!("> ");
    let _ = std::io::stdout().flush();
    let reset_position = read_position()?;
    let mut buffer = String::new();
    loop {
        if let Ok(c) = read_char() {
            if c == 1 {
                // CTRL-A
                let _ = goto_position(reset_position.0, reset_position.1);
            } else if c == 3 {
                // CTRL-C
                break;
            } else if c == 13 {
                // Carriage return
                print!("\r\n");
                let _ = std::io::stdout().flush();

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
            } else if c == 127 {
                buffer.pop();
                paint_string(&buffer, reset_position.0, reset_position.1)?;
            } else {
                buffer.push(c as char);
                paint_string(&buffer, reset_position.0, reset_position.1)?;
            }
        }
    }

    let _ = term.disable_raw_mode();

    Ok(())
}

fn main() {
    let _ = term_loop();
}
