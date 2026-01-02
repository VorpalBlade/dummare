# Changelog

All notable changes to this project will be documented in this file.
Keep in mind that this is only updated when releases are made and the file
is generated automatically from commit messages (and may or may not be lightly
edited).

For a possibly more edited message focused on the binary please see the github
releases.

## [0.1.2] - 2026-01-02

### Features

- Use terminfo to detect if terminal is hard copy
- Use terminfo width as fallback if ioctl fails

### Documentation

- Add links to crates.io/lib.rs & add keywords

### Other stuff

- Update outdated code comments


## [0.1.1] - 2026-01-02

### Features

- Add bold and underline emulation for hard copy terminals
- Use terminfo database and ANSI handler

### Refactoring

- Remove un-needed LineWriter wrapper

### Other stuff

- *(ci)* Set environment
- *(ci)* Set up trusted publishing
- *(ci)* Fix dependabot commit message format


## [0.1.0] - 2026-01-01

Initial version
