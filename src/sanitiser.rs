// Adapted from `strip-ansi-escapes` crate, but removed unused parts and
// expanded on execute().
use std::io::Write;
use vte::Parser;
use vte::Perform;

/// A writer that sanitises the output.
pub(crate) struct Writer<W>
where
    W: Write,
{
    performer: Performer<W>,
    parser: Parser,
}

impl<W> Writer<W>
where
    W: Write,
{
    pub fn new(inner: W) -> Self {
        Self {
            performer: Performer {
                writer: inner,
                err: None,
            },
            parser: Parser::new(),
        }
    }
}

impl<W> Write for Writer<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.parser.advance(&mut self.performer, buf);
        match self.performer.err.take() {
            Some(e) => Err(e),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.performer.flush()
    }
}

struct Performer<W>
where
    W: Write,
{
    writer: W,
    err: Option<std::io::Error>,
}

impl<W> Performer<W>
where
    W: Write,
{
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
impl<W> Perform for Performer<W>
where
    W: Write,
{
    fn print(&mut self, c: char) {
        // Just print bytes to the inner writer.
        self.err = write!(self.writer, "{c}").err();
    }

    fn execute(&mut self, byte: u8) {
        // Filter these
        match byte {
            b'\n' | b'\r' | b'\t' | BELL | BACKSPACE => {
                self.err = self.writer.write_all(&[byte]).err();
            }
            b => eprintln!("WARN: Unhandled control byte: {b}"),
        }
    }
}

const BELL: u8 = 0x07;
// This probably won't work for actual hard copy terminals?
const BACKSPACE: u8 = 0x08;
