//! # r68k - Motorola 68000 CPU Emulator
//!
//! A cycle-accurate Motorola 68000 CPU emulator, ported from Karl Stenerud's
//! battle-tested [Musashi](https://github.com/kstenerud/Musashi) emulator
//! which has been successfully running in the MAME project for years.
//!
//! ## Features
//!
//! - Complete MC68000 instruction set implementation
//! - Cycle-accurate emulation verified against Musashi via property-based testing
//! - All addressing modes supported (Dn, An, (An), (An)+, -(An), (d,An), (d,An,Xn), etc.)
//! - Full exception handling (Address Error, Illegal Instruction, Traps, Interrupts)
//! - Supervisor/User mode with separate stack pointers
//! - Customizable memory interface via the [`AddressBus`] trait
//! - Customizable interrupt handling via the [`InterruptController`] trait
//! - Zero production dependencies
//!
//! ## Quick Start
//!
//! ```rust
//! use r68k::cpu::ConfiguredCore;
//! use r68k::ram::PagedMem;
//! use r68k::interrupts::AutoInterruptController;
//!
//! // Create memory with 0x00 as the uninitialized byte pattern
//! let mut memory = PagedMem::new(0x00000000);
//!
//! // Write reset vectors: SSP at 0x00, PC at 0x04
//! // Initial stack pointer
//! memory.write_u8(0x00, 0x00);
//! memory.write_u8(0x01, 0x01);
//! memory.write_u8(0x02, 0x00);
//! memory.write_u8(0x03, 0x00); // SSP = 0x00010000
//!
//! // Initial program counter
//! memory.write_u8(0x04, 0x00);
//! memory.write_u8(0x05, 0x00);
//! memory.write_u8(0x06, 0x10);
//! memory.write_u8(0x07, 0x00); // PC = 0x00001000
//!
//! // Write a NOP instruction at 0x1000
//! memory.write_u8(0x1000, 0x4E);
//! memory.write_u8(0x1001, 0x71); // NOP = 0x4E71
//!
//! // Create CPU with autovectored interrupt controller
//! let mut cpu = ConfiguredCore::new_with(
//!     0,
//!     AutoInterruptController::new(),
//!     memory
//! );
//!
//! // Reset the CPU (loads SSP and PC from vectors)
//! cpu.reset();
//!
//! // Execute instructions for up to 1000 cycles
//! let cycles_used = cpu.execute(1000);
//! ```
//!
//! ## Custom Memory Implementation
//!
//! Implement [`AddressBus`] to provide your own memory system:
//!
//! ```rust
//! use r68k::ram::{AddressBus, AddressSpace};
//!
//! struct MyMemory {
//!     rom: Vec<u8>,
//!     ram: Vec<u8>,
//! }
//!
//! impl AddressBus for MyMemory {
//!     fn copy_from(&mut self, other: &Self) {
//!         self.ram.copy_from_slice(&other.ram);
//!     }
//!
//!     fn read_byte(&self, _space: AddressSpace, address: u32) -> u32 {
//!         let addr = address as usize & 0xFFFFFF; // 24-bit address bus
//!         if addr < 0x8000 {
//!             self.rom.get(addr).copied().unwrap_or(0) as u32
//!         } else {
//!             self.ram.get(addr - 0x8000).copied().unwrap_or(0) as u32
//!         }
//!     }
//!
//!     fn read_word(&self, space: AddressSpace, address: u32) -> u32 {
//!         (self.read_byte(space, address) << 8)
//!             | self.read_byte(space, address.wrapping_add(1))
//!     }
//!
//!     fn read_long(&self, space: AddressSpace, address: u32) -> u32 {
//!         (self.read_word(space, address) << 16)
//!             | self.read_word(space, address.wrapping_add(2))
//!     }
//!
//!     fn write_byte(&mut self, _space: AddressSpace, address: u32, value: u32) {
//!         let addr = (address as usize & 0xFFFFFF).saturating_sub(0x8000);
//!         if let Some(byte) = self.ram.get_mut(addr) {
//!             *byte = value as u8;
//!         }
//!     }
//!
//!     fn write_word(&mut self, space: AddressSpace, address: u32, value: u32) {
//!         self.write_byte(space, address, value >> 8);
//!         self.write_byte(space, address.wrapping_add(1), value);
//!     }
//!
//!     fn write_long(&mut self, space: AddressSpace, address: u32, value: u32) {
//!         self.write_word(space, address, value >> 16);
//!         self.write_word(space, address.wrapping_add(2), value);
//!     }
//! }
//! ```
//!
//! ## Exception Handling
//!
//! Use [`Callbacks`](cpu::Callbacks) to intercept exceptions:
//!
//! ```rust
//! use r68k::cpu::{Callbacks, Core, Cycles, Exception, Result};
//!
//! struct MyCallbacks;
//!
//! impl Callbacks for MyCallbacks {
//!     fn exception_callback(&mut self, core: &mut impl Core, ex: Exception) -> Result<Cycles> {
//!         match ex {
//!             Exception::Trap(num, _) => {
//!                 println!("TRAP #{} called", num - 32);
//!                 // Return cycles consumed, or Err(ex) to let CPU handle it
//!                 Ok(Cycles(40))
//!             }
//!             _ => Err(ex), // Let CPU handle other exceptions normally
//!         }
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! - [`cpu`] - CPU emulation core with [`ConfiguredCore`](cpu::ConfiguredCore) and [`Core`](cpu::Core) trait
//! - [`ram`] - Memory interface with [`AddressBus`] trait and [`PagedMem`](ram::PagedMem) implementation
//! - [`interrupts`] - Interrupt handling with [`InterruptController`] trait
//! - [`common`] - Shared constants and opcode definitions
//!
//! [`AddressBus`]: ram::AddressBus
//! [`InterruptController`]: interrupts::InterruptController

pub mod common;
pub mod cpu;
pub mod ram;
pub mod interrupts;

// Re-export commonly used types at crate root for convenience
pub use cpu::{Cpu, ConfiguredCore, Core, Cycles, Callbacks, Exception, ProcessingState, Result};
pub use ram::{AddressBus, AddressSpace, PagedMem, SUPERVISOR_DATA, SUPERVISOR_PROGRAM, USER_DATA, USER_PROGRAM};
pub use interrupts::{InterruptController, AutoInterruptController};

// Musashi integration tests - require external Musashi C library
#[cfg(all(test, feature = "musashi"))]
pub mod musashi;
