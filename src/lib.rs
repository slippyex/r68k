//! r68k - Motorola 68000 CPU emulator
//!
//! A Rust port of Karl Stenerud's Musashi emulator, which has been
//! successfully running in the MAME project for years.

pub mod common;
pub mod cpu;
pub mod ram;
pub mod interrupts;

// Musashi integration tests - require external Musashi C library
#[cfg(all(test, feature = "musashi"))]
pub mod musashi;
