// Taken from `pty-process` examples.
use std::os::fd::AsFd as _;

/// RAII guard to set terminal to raw mode and restore on drop.
pub struct RawGuard {
    termios: nix::sys::termios::Termios,
}

impl RawGuard {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let stdin = std::io::stdin();
        let stdin = stdin.as_fd();
        let termios =
            nix::sys::termios::tcgetattr(stdin).expect("Failed to get terminal attributes");
        let mut termios_raw = termios.clone();
        nix::sys::termios::cfmakeraw(&mut termios_raw);
        nix::sys::termios::tcsetattr(stdin, nix::sys::termios::SetArg::TCSANOW, &termios_raw)
            .expect("Failed to set terminal to raw mode");
        Self { termios }
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        let stdin = std::io::stdin();
        let stdin = stdin.as_fd();
        let _ =
            nix::sys::termios::tcsetattr(stdin, nix::sys::termios::SetArg::TCSANOW, &self.termios);
    }
}
