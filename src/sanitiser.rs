// Adapted from `strip-ansi-escapes` crate, but removed unused parts and
// expanded on execute() and changed to full ANSI handling.
use modular_bitfield::bitfield;
use modular_bitfield::prelude::B6;
use std::io::Write;
use terminfo::Database;
use terminfo::capability;
use vte::ansi::Handler;
use vte::ansi::Processor;

/// A writer that sanitises the output.
pub(crate) struct Writer<W>
where
    W: Write,
{
    handler: Sanitizer<W>,
    parser: Processor,
}

impl<W> Writer<W>
where
    W: Write,
{
    pub fn new(inner: W, terminfo: Database) -> Self {
        Self {
            handler: Sanitizer {
                err: None,
                terminfo,
                writer: inner,
                attributes: Attributes::new(),
            },
            parser: Processor::new(),
        }
    }
}

impl<W> Write for Writer<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.parser.advance(&mut self.handler, buf);
        match self.handler.err.take() {
            Some(e) => Err(e),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.handler.flush()
    }
}

struct Sanitizer<W>
where
    W: Write,
{
    err: Option<std::io::Error>,
    terminfo: Database,
    writer: W,
    attributes: Attributes,
}

/// Attributes that we can emulate on hard copy terminals.
#[bitfield]
struct Attributes {
    bold: bool,
    underline: bool,
    #[allow(dead_code)]
    reserved: B6,
}

impl<W> Sanitizer<W>
where
    W: Write,
{
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    fn move_right(&mut self, count: usize) {
        if let Some(e) = self.terminfo.get::<capability::CursorRight<'_>>() {
            for _ in 0..count {
                self.err = terminfo::expand!(&mut self.writer, e.as_ref())
                    .map_err(terminfo_err_mapper)
                    .err();
            }
        } else {
            // Fallback: spaces (should work for hard copy terminals)
            for _ in 0..count {
                self.err = self.writer.write_all(b" ").err();
            }
        }
    }
}

macro_rules! expand_cap {
    ($self:ident, $cap:ty) => {
        if let Some(e) = $self.terminfo.get::<$cap>() {
            $self.err = terminfo::expand!(&mut $self.writer, e.as_ref())
                .map_err(terminfo_err_mapper)
                .err();
        }
    };
    ($self:ident, $cap:ty => $tracker:ident $value:ident) => {
        if let Some(e) = $self.terminfo.get::<$cap>() {
            $self.err = terminfo::expand!(&mut $self.writer, e.as_ref())
                .map_err(terminfo_err_mapper)
                .err();
        } else {
            pastey::paste! {
                $self.attributes.[<set_ $tracker>]($value);
            }
        }
    };
    ($self:ident, $cap:ty, $count:expr) => {
        if let Some(e) = $self.terminfo.get::<$cap>() {
            for _ in 0..$count {
                $self.err = terminfo::expand!(&mut $self.writer, e.as_ref())
                    .map_err(terminfo_err_mapper)
                    .err();
            }
        }
    };
}

impl<W> Handler for Sanitizer<W>
where
    W: Write,
{
    fn input(&mut self, c: char) {
        // Do the underline first, so that the text is still there if the terminal *can*
        // do erases.
        if self.attributes.underline() {
            self.err = write!(self.writer, "_\x08").err();
        }
        self.err = write!(self.writer, "{c}").err();
        if self.attributes.bold() {
            self.err = write!(self.writer, "\x08{c}").err();
        }
    }

    fn goto_col(&mut self, column: usize) {
        // We don't keep track of where we are, but we can fake this by CR followed by
        // spaces
        expand_cap!(self, capability::CarriageReturn<'_>);
        self.move_right(column);
    }

    fn insert_blank(&mut self, count: usize) {
        for _ in 0..count {
            self.err = self.writer.write_all(b" ").err();
        }
        // TODO: Should cursor be reset?
    }

    fn move_up(&mut self, rows: usize) {
        expand_cap!(self, capability::CursorUp<'_>, rows);
    }

    fn move_down(&mut self, rows: usize) {
        expand_cap!(self, capability::CursorDown<'_>, rows);
    }

    fn move_forward(&mut self, cols: usize) {
        self.move_right(cols);
    }

    fn move_backward(&mut self, cols: usize) {
        expand_cap!(self, capability::CursorLeft<'_>, cols);
    }

    fn move_down_and_cr(&mut self, rows: usize) {
        expand_cap!(self, capability::CursorDown<'_>, rows);
        expand_cap!(self, capability::CarriageReturn<'_>);
    }

    fn move_up_and_cr(&mut self, _row: usize) {
        expand_cap!(self, capability::CursorUp<'_>, _row);
        expand_cap!(self, capability::CarriageReturn<'_>);
    }

    fn put_tab(&mut self, count: u16) {
        expand_cap!(self, capability::Tab<'_>, count as usize);
    }

    fn backspace(&mut self) {
        self.err = self.writer.write_all(&[BACKSPACE]).err();
    }

    fn carriage_return(&mut self) {
        expand_cap!(self, capability::CarriageReturn<'_>);
    }

    fn linefeed(&mut self) {
        self.err = self.writer.write_all(b"\n").err();
    }

    fn bell(&mut self) {
        expand_cap!(self, capability::Bell<'_>);
    }

    fn substitute(&mut self) {
        self.err = self.writer.write_all(&[SUBSTITUTE]).err();
    }

    fn newline(&mut self) {
        expand_cap!(self, capability::CarriageReturn<'_>);
        self.linefeed();
    }

    fn scroll_up(&mut self, rows: usize) {
        expand_cap!(self, capability::ScrollReverse<'_>, rows);
    }

    fn scroll_down(&mut self, rows: usize) {
        expand_cap!(self, capability::ScrollForward<'_>, rows);
    }

    fn insert_blank_lines(&mut self, rows: usize) {
        // TODO: We might not want this (if it is used for TUIs mostly)
        for _ in 0..rows {
            self.err = self.writer.write_all(b"\n").err();
        }
    }

    // TODO: This would be nice, but hard copy terminals lack rs1-rs3
    fn reset_state(&mut self) {}

    fn terminal_attribute(&mut self, attr: vte::ansi::Attr) {
        match attr {
            vte::ansi::Attr::Reverse => {
                expand_cap!(self, capability::EnterStandoutMode<'_>);
            }
            vte::ansi::Attr::CancelReverse => {
                expand_cap!(self, capability::ExitStandoutMode<'_>);
            }
            vte::ansi::Attr::Reset => {
                expand_cap!(self, capability::ExitAttributeMode<'_>);
                self.attributes.set_bold(false);
                self.attributes.set_underline(false);
            }
            vte::ansi::Attr::Bold => {
                expand_cap!(self, capability::EnterBoldMode<'_> => bold true);
            }
            vte::ansi::Attr::CancelBold => {
                // This code is sometimes "double underline" sometimes "cancel bold",
                // so in practice it isn't really used. And there is no terminfo code for it.
                self.attributes.set_bold(false);
            }
            vte::ansi::Attr::Underline => {
                expand_cap!(self, capability::EnterUnderlineMode<'_> => underline true);
            }
            vte::ansi::Attr::CancelUnderline => {
                expand_cap!(self, capability::ExitUnderlineMode<'_>);
                self.attributes.set_underline(false);
            }
            vte::ansi::Attr::Italic => {
                expand_cap!(self, capability::EnterItalicsMode<'_>);
            }
            vte::ansi::Attr::CancelItalic => {
                expand_cap!(self, capability::ExitItalicsMode<'_>);
            }
            // These are not implemented
            vte::ansi::Attr::Foreground(_color) => (),
            vte::ansi::Attr::Background(_color) => (),
            vte::ansi::Attr::UnderlineColor(_color) => (),
            vte::ansi::Attr::DoubleUnderline => (),
            vte::ansi::Attr::Undercurl => (),
            vte::ansi::Attr::DottedUnderline => (),
            vte::ansi::Attr::DashedUnderline => (),
            vte::ansi::Attr::BlinkSlow => (),
            vte::ansi::Attr::BlinkFast => (),
            vte::ansi::Attr::Hidden => (),
            vte::ansi::Attr::Strike => (),
            vte::ansi::Attr::Dim => (),
            vte::ansi::Attr::CancelBoldDim => (),
            vte::ansi::Attr::CancelBlink => (),
            vte::ansi::Attr::CancelHidden => (),
            vte::ansi::Attr::CancelStrike => (),
        }
    }
}

// This probably won't work for actual hard copy terminals?
const BACKSPACE: u8 = 0x08;
const SUBSTITUTE: u8 = 0x1A;

fn terminfo_err_mapper(e: terminfo::Error) -> std::io::Error {
    match e {
        terminfo::Error::Io(error) => error,
        terminfo::Error::NotFound => {
            std::io::Error::new(std::io::ErrorKind::NotFound, "terminfo database not found")
        }
        terminfo::Error::Parse => std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "failed to parse terminfo database",
        ),
        terminfo::Error::Expand(expand) => {
            std::io::Error::other(format!("terminfo expansion error: {expand:?}"))
        }
    }
}
