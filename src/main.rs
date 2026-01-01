mod raw_guard;
mod sanitiser;

use anyhow::Context;
use clap::Parser as _;
use std::io::Read as _;
use std::io::Write as _;
use std::os::fd::AsFd as _;
use std::process::ExitCode;

mod cli {
    use clap_derive::Parser;

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    /// Dumbifies your terminal, for all your hard copy terminal needs
    pub struct Cli {
        /// TERM to set to inner process
        #[arg(short, long, default_value = "dumb")]
        pub term: String,
        /// Number of columns to use [default: parent terminal width, fallback:
        /// 80]
        #[arg(short, long)]
        pub cols: Option<u16>,
        /// Command to run inside the terminal
        #[arg()]
        pub command: String,
    }
}

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    let width = args.cols.unwrap_or_else(|| {
        terminal_size::terminal_size()
            .map(|(terminal_size::Width(w), _)| w)
            .unwrap_or(80)
    });
    let (mut pty, pts) = pty_process::blocking::open().context("Failed to open PTY")?;
    pty.resize(pty_process::Size::new(24, width))
        .context("Failed to set size of PTY")?;

    let mut child = pty_process::blocking::Command::new(&args.command)
        .env("TERM", &args.term)
        .env("NO_COLOR", "1") // Some commands respect this, which should help a bit.
        .spawn(pts)
        .with_context(|| format!("Failed to spawn command: {}", &args.command))?;

    run(&mut child, &mut pty).context("Error during PTY I/O")?;

    Ok(())
}

// Adapted from example in `pty-process` crate.
fn run(
    child: &mut std::process::Child,
    pty: &mut pty_process::blocking::Pty,
) -> anyhow::Result<ExitCode> {
    // This enables raw mode on stdin, and restores the previous mode on drop.
    // It is needed to not get a "local" echo of what the user types.
    let _raw = raw_guard::RawGuard::new();
    let mut buf = [0_u8; 4096];
    let stdin = std::io::stdin();
    let stdin_fd = stdin.as_fd();
    let mut stdin = stdin.lock();
    let mut stdout = std::io::stdout().lock();
    let mut sanitiser = sanitiser::Writer::new(&mut stdout);

    loop {
        let mut set = nix::sys::select::FdSet::new();
        set.insert(pty.as_fd());
        set.insert(stdin_fd);
        match nix::sys::select::select(None, Some(&mut set), None, None, None) {
            Ok(n) => {
                if n > 0 {
                    let pty_ready = set.contains(pty.as_fd());
                    let stdin_ready = set.contains(stdin_fd);
                    if pty_ready {
                        match pty.read(&mut buf) {
                            Ok(bytes) => {
                                let buf = &buf[..bytes];
                                sanitiser
                                    .write_all(buf)
                                    .context("write of pty to stdout failed")?;
                                sanitiser.flush().context("flush of stdout failed")?;
                            }
                            Err(e) => {
                                match child.try_wait() {
                                    Ok(Some(code)) => {
                                        return Ok(ExitCode::from(code.code().unwrap_or(1) as u8));
                                    }
                                    Ok(None) => {
                                        anyhow::bail!("pty read failed while child is alive: {e:?}")
                                    }
                                    Err(err) => anyhow::bail!(
                                        "wait after IO error failed: {err:?}, original error: \
                                         {e:?}"
                                    ),
                                };
                            }
                        };
                    }
                    if stdin_ready {
                        match stdin.read(&mut buf) {
                            Ok(bytes) => {
                                let buf = &buf[..bytes];
                                pty.write_all(buf).context("write of stdin to pty failed")?;
                            }
                            Err(e) => {
                                anyhow::bail!("stdin read failed: {e:?}");
                            }
                        }
                    }
                }
            }
            Err(e) => anyhow::bail!("select() failed with: {e:?}"),
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => {
                anyhow::bail!("wait failed: {e:?}");
            }
        }
    }
    unreachable!();
}
