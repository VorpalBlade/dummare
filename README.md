# dummare

[ [lib.rs] ] [ [crates.io] ]

Strips escape codes and such nonsense from terminal output to make it suitable for hard copy terminals (i.e. paper based terminals).

This is extremely niche. If you don't know why you would want this, you probably don't.

## Technical details

This emulates a terminal using PTYs, and uses a terminal parsing library to clean the output
of everything that isn't plain text or a few allowlisted control characters (CR, LF, tab, bell, backspace).

It will work for simple use cases like progress bars and coloured text, but it won't handle
complex full screen programs at all. Don't expect to run vi or emacs.

## Building etc

This is a Rust project built with Cargo. If you just want to use it, download a prebuilt binary from
the releases page instead. If you want to contribute, standard Rust project structure applies.

However, if you need to build this for an obscure platform or such:

1. Clone this repository.
2. Make sure you have Rust and Cargo installed. (At least 1.92 as of writing this.)
3. Run `cargo build --release` to build the project.
4. The resulting binary will be in `target/release/dummare` and can be copied to somewhere in your PATH.

## Usage

Run `dummare --help` to see usage information. Typical usage would be `dummare bash` to run bash
with output cleaned for a dumb terminal.

Remember to *properly* set your `TERM` in the parent environment. This is used to
detect what control codes your terminal supports (if any), if your terminal is a
real hard copy terminal or not, and for the terminal width.

## Credits

A lot of the code was adapted from examples in the `pty-process` (MIT) crate.
Some code was adapted from core library code in `strip-ansi-escapes` (Apache-2.0 or MIT).

See comments in individual source files for the details.

Thank you to the authors or those projects.

## Name

"dummare" is Swedish for "dumber", and since this adapts your terminal for use with dumb terminals,
that seemed like a fitting name.

[crates.io]: https://crates.io/crates/dummare
[lib.rs]: https://lib.rs/crates/dummare
