//! Memory interface for the 68000 CPU.
//!
//! This module provides the [`AddressBus`] trait for implementing custom memory systems,
//! along with a ready-to-use [`PagedMem`] implementation.
//!
//! # Address Space
//!
//! The 68000 has a 24-bit address bus, addressing up to 16 MB of memory.
//! Memory accesses are qualified by an [`AddressSpace`] indicating:
//! - Whether the CPU is in supervisor or user mode
//! - Whether the access is for program (instruction fetch) or data
//!
//! # Implementing Custom Memory
//!
//! Implement [`AddressBus`] to provide your own memory system:
//!
//! ```rust
//! use r68k::ram::{AddressBus, AddressSpace};
//!
//! struct SimpleMemory {
//!     data: Vec<u8>,
//! }
//!
//! impl AddressBus for SimpleMemory {
//!     fn copy_from(&mut self, other: &Self) {
//!         self.data.copy_from_slice(&other.data);
//!     }
//!
//!     fn read_byte(&self, _space: AddressSpace, address: u32) -> u32 {
//!         self.data.get(address as usize).copied().unwrap_or(0) as u32
//!     }
//!
//!     fn read_word(&self, space: AddressSpace, address: u32) -> u32 {
//!         (self.read_byte(space, address) << 8) | self.read_byte(space, address + 1)
//!     }
//!
//!     fn read_long(&self, space: AddressSpace, address: u32) -> u32 {
//!         (self.read_word(space, address) << 16) | self.read_word(space, address + 2)
//!     }
//!
//!     fn write_byte(&mut self, _space: AddressSpace, address: u32, value: u32) {
//!         if let Some(byte) = self.data.get_mut(address as usize) {
//!             *byte = value as u8;
//!         }
//!     }
//!
//!     fn write_word(&mut self, space: AddressSpace, address: u32, value: u32) {
//!         self.write_byte(space, address, value >> 8);
//!         self.write_byte(space, address + 1, value);
//!     }
//!
//!     fn write_long(&mut self, space: AddressSpace, address: u32, value: u32) {
//!         self.write_word(space, address, value >> 16);
//!         self.write_word(space, address + 2, value);
//!     }
//! }
//! ```

pub mod loggingmem;
pub mod pagedmem;
pub use self::pagedmem::PagedMem;

/// Mask for the 24-bit address bus (16 MB addressable space).
pub const ADDRBUS_MASK: u32 = 0x00ff_ffff;

/// Represents the address space qualifier for memory accesses.
///
/// The 68000 uses function codes to distinguish between different types
/// of memory accesses. This type encodes the CPU mode (supervisor/user)
/// and access type (program/data).
///
/// # Predefined Constants
///
/// - [`SUPERVISOR_PROGRAM`]: Supervisor mode, program access (FC=6)
/// - [`SUPERVISOR_DATA`]: Supervisor mode, data access (FC=5)
/// - [`USER_PROGRAM`]: User mode, program access (FC=2)
/// - [`USER_DATA`]: User mode, data access (FC=1)
#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct AddressSpace(Mode, Segment);

impl AddressSpace {
    /// Returns the function code (FC) for this address space.
    ///
    /// Function codes are 3-bit values output by the 68000:
    /// - 1: User data
    /// - 2: User program
    /// - 5: Supervisor data
    /// - 6: Supervisor program
    pub fn fc(self) -> u32 {
        match self {
            USER_DATA => 1,
            USER_PROGRAM => 2,
            SUPERVISOR_DATA => 5,
            SUPERVISOR_PROGRAM => 6,
        }
    }
}
use std::fmt;
impl fmt::Debug for AddressSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AddressSpace(mode, segment) => write!(f, "[{mode:?}/{segment:?}]"),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum Segment {
    Program, Data
}
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum Mode {
    User, Supervisor
}

/// Supervisor mode, program access (instruction fetch). Function code 6.
pub const SUPERVISOR_PROGRAM: AddressSpace = AddressSpace(Mode::Supervisor, Segment::Program);
/// Supervisor mode, data access. Function code 5.
pub const SUPERVISOR_DATA: AddressSpace = AddressSpace(Mode::Supervisor, Segment::Data);
/// User mode, program access (instruction fetch). Function code 2.
pub const USER_PROGRAM: AddressSpace = AddressSpace(Mode::User, Segment::Program);
/// User mode, data access. Function code 1.
pub const USER_DATA: AddressSpace = AddressSpace(Mode::User, Segment::Data);

/// Trait for implementing the 68000's memory interface.
///
/// Implement this trait to provide custom memory for the CPU emulator.
/// The trait requires methods for reading and writing bytes, words (16-bit),
/// and longs (32-bit) at any address.
///
/// # Big-Endian Byte Order
///
/// The 68000 is a big-endian processor. For word and long accesses:
/// - The most significant byte is at the lower address
/// - Example: Writing 0x1234 to address 0x100 stores 0x12 at 0x100 and 0x34 at 0x101
///
/// # Address Alignment
///
/// Word and long accesses to odd addresses cause an Address Error exception
/// in the CPU. The memory implementation does not need to check for this;
/// the CPU handles alignment checking.
///
/// # Address Space
///
/// The `address_space` parameter indicates the type of access. Many systems
/// ignore this and provide a flat memory model, but it can be used to
/// implement memory protection or separate program/data spaces.
pub trait AddressBus {
    /// Copies memory contents from another instance.
    ///
    /// Used for cloning CPU state including memory.
    fn copy_from(&mut self, other: &Self);

    /// Reads a byte (8-bit) from the given address.
    ///
    /// Returns the byte value zero-extended to u32.
    fn read_byte(&self, address_space: AddressSpace, address: u32) -> u32;

    /// Reads a word (16-bit) from the given address.
    ///
    /// Returns the word value zero-extended to u32, in big-endian order.
    fn read_word(&self, address_space: AddressSpace, address: u32) -> u32;

    /// Reads a long (32-bit) from the given address.
    ///
    /// Returns the long value in big-endian order.
    fn read_long(&self, address_space: AddressSpace, address: u32) -> u32;

    /// Writes a byte (8-bit) to the given address.
    ///
    /// Only the lower 8 bits of `value` are used.
    fn write_byte(&mut self, address_space: AddressSpace, address: u32, value: u32);

    /// Writes a word (16-bit) to the given address.
    ///
    /// Only the lower 16 bits of `value` are used, written in big-endian order.
    fn write_word(&mut self, address_space: AddressSpace, address: u32, value: u32);

    /// Writes a long (32-bit) to the given address.
    ///
    /// Written in big-endian order.
    fn write_long(&mut self, address_space: AddressSpace, address: u32, value: u32);

    /// Called when a RESET instruction is executed.
    ///
    /// Override this to reset external devices connected to the bus.
    /// Default implementation does nothing.
    fn reset_instruction(&mut self) {}

    /// Returns additional wait state cycles for a memory access.
    ///
    /// Override this to model bus timing characteristics of specific hardware.
    /// For example, the Atari ST has wait states when accessing ROM or I/O.
    ///
    /// # Arguments
    ///
    /// * `address` - The memory address being accessed
    /// * `access_size` - Size of access: 1 (byte), 2 (word), or 4 (long)
    /// * `is_write` - True for write operations, false for reads
    ///
    /// # Returns
    ///
    /// Additional cycles to add to the access (0 = no wait states).
    /// Default implementation returns 0 for all accesses.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn wait_cycles(&self, address: u32, access_size: u8, is_write: bool) -> i32 {
    ///     match address {
    ///         0xFC0000..=0xFFFFFF => 2,  // ROM: 2 wait states
    ///         0xFF8000..=0xFF8FFF => 4,  // I/O: 4 wait states
    ///         _ => 0,                     // RAM: no wait states
    ///     }
    /// }
    /// ```
    fn wait_cycles(&self, _address: u32, _access_size: u8, _is_write: bool) -> i32 {
        0
    }
}

