# r68k

[![Crates.io](https://img.shields.io/crates/v/r68k.svg)](https://crates.io/crates/r68k)
[![Documentation](https://docs.rs/r68k/badge.svg)](https://docs.rs/r68k)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

r68k is an emulator for the Motorola 68000 CPU written in Rust, ported from [Karl Stenerud's Musashi](https://github.com/kstenerud/Musashi). Musashi "has been successfully running in the MAME project (www.mame.net) for years and so has had time to mature." - so unlike most other emulators Musashi is of proven quality to run complex real-world m68k software, which makes it a solid foundation.

## Features

- Complete 68000 instruction set implementation
- Cycle-accurate emulation verified against Musashi
- Accurate exception timing (per MC68000UM Table 8-14)
- Configurable cycle granularity (e.g., 4-cycle boundary for Atari ST)
- Configurable memory wait states for ROM/I/O timing
- Support for autovectored interrupts
- STOP and HALT states properly emulated
- Host callbacks for RESET instruction and exception overrides
- Flexible memory interface via `AddressBus` trait
- No external dependencies for the core emulator

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
r68k = "0.2"
```

## Quick Start

```rust
use r68k::cpu::{ConfiguredCore, ProcessingState};
use r68k::interrupts::AutoInterruptController;
use r68k::ram::{AddressBus, AddressSpace};

// Implement the AddressBus trait for your memory system
#[derive(Clone)]
struct SimpleMemory {
    ram: Vec<u8>,
}

impl AddressBus for SimpleMemory {
    fn copy_from(&mut self, other: &Self) {
        self.ram = other.ram.clone();
    }

    fn read_byte(&self, _space: AddressSpace, addr: u32) -> u32 {
        self.ram.get(addr as usize).copied().unwrap_or(0) as u32
    }

    fn read_word(&self, space: AddressSpace, addr: u32) -> u32 {
        let hi = self.read_byte(space, addr);
        let lo = self.read_byte(space, addr.wrapping_add(1));
        (hi << 8) | lo
    }

    fn read_long(&self, space: AddressSpace, addr: u32) -> u32 {
        let hi = self.read_word(space, addr);
        let lo = self.read_word(space, addr.wrapping_add(2));
        (hi << 16) | lo
    }

    fn write_byte(&mut self, _space: AddressSpace, addr: u32, value: u32) {
        if let Some(cell) = self.ram.get_mut(addr as usize) {
            *cell = value as u8;
        }
    }

    fn write_word(&mut self, space: AddressSpace, addr: u32, value: u32) {
        self.write_byte(space, addr, value >> 8);
        self.write_byte(space, addr.wrapping_add(1), value & 0xFF);
    }

    fn write_long(&mut self, space: AddressSpace, addr: u32, value: u32) {
        self.write_word(space, addr, value >> 16);
        self.write_word(space, addr.wrapping_add(2), value & 0xFFFF);
    }
}

fn main() {
    let mem = SimpleMemory { ram: vec![0; 65536] };
    let int_ctrl = AutoInterruptController::new();
    let mut cpu = ConfiguredCore::new_with(0x1000, int_ctrl, mem);

    // CPU starts in exception state, set to Normal to execute
    cpu.processing_state = ProcessingState::Normal;

    // Execute one instruction
    let cycles = cpu.execute1();
    println!("Executed instruction in {} cycles", cycles.0);
}
```

## The Processor

The [Motorola 68000](https://en.wikipedia.org/wiki/Motorola_68000) CPU, commonly referred to as m68k, was a very successful CPU introduced in 1979 that powered several classic personal computers of the 1980s, such as the Apple Macintosh, Commodore Amiga and Atari ST, as well as the first SUN and Apollo UNIX workstations. It was used in several arcade machines and game consoles such as the Sega Genesis/Mega Drive.

It typically ran at 8MHz and could address up to 16MB of RAM.

## Usage

The emulator is not a full computer system emulation - it's just a CPU connected to memory via the `AddressBus` trait. You load memory with a program (a series of bytes representing valid instructions and data), set the program counter, and execute instructions one by one.

One can build a complete computer emulation on top of r68k by implementing:
- Memory-mapped I/O via the `AddressBus` trait
- Interrupt controllers
- Peripheral devices

## Cycle Granularity

Some systems synchronize the 68000's bus cycles to external hardware. The Atari ST, for example, synchronizes memory access to the video shifter running at 2 MHz, resulting in all bus cycles being aligned to 4-cycle boundaries (8 MHz / 2 MHz = 4).

This is important for cycle-accurate emulation of:
- Demoscene effects (raster tricks, fullscreen, border removal)
- Sync buzzer audio (software PCM via YM2149)
- Any timing-critical code

Configure cycle granularity after creating the CPU:

```rust
let mut cpu = ConfiguredCore::new_with(0x1000, int_ctrl, mem);
cpu.set_cycle_granularity(4); // Atari ST mode

// Instructions now report cycles aligned to 4-cycle boundaries:
// - 6 cycles -> 8 cycles
// - 18 cycles -> 20 cycles
let cycles = cpu.execute1();
```

The default granularity is 1 (no alignment).

## Exception Timing

Exception processing costs CPU cycles according to MC68000UM Table 8-14. This is critical for cycle-accurate emulation, especially for interrupt-driven audio (sync buzzer, digidrum) where the interrupt handler timing directly affects sample rates.

| Exception | Cycles | Bus Cycles (R/W) |
|-----------|--------|------------------|
| Address Error | 50 | 4/7 |
| Illegal Instruction | 34 | 4/3 |
| Privilege Violation | 34 | 4/3 |
| TRAP #n | 38 | 4/3 |
| TRAPV | 34 | 4/3 |
| Divide by Zero | 42 + EA | 5/3 |
| CHK | 44 + EA | 5/3 |
| Interrupt | 44 | 5/3 |

These cycle counts include stacking SR/PC, fetching the exception vector, and prefetching the first two instruction words of the handler.

## Wait States

Memory accesses can have additional wait state cycles depending on the hardware. Override the `wait_cycles` method in your `AddressBus` implementation to model this:

```rust
impl AddressBus for AtariSTMemory {
    // ... other methods ...

    fn wait_cycles(&self, address: u32, access_size: u8, is_write: bool) -> i32 {
        match address {
            0xFC0000..=0xFFFFFF => 2,  // ROM: 2 wait states
            0xFF8000..=0xFF8FFF => 4,  // I/O registers: 4 wait states
            _ => 0,                     // RAM: no wait states
        }
    }
}
```

Wait states are accumulated during instruction execution and added to the instruction's base cycle count before cycle granularity alignment is applied. The default implementation returns 0 for all accesses.

## CPU Emulator Status

The r68k emulator implements the original 68000 instruction set. It does not support instructions specific to newer CPUs in the 68k family (68010, 68020, 68040) at this time.

- All instructions implemented and verified against Musashi
- Autovectored, auto-resetting interrupts
- STOP and HALT states properly emulated
- Host callbacks for RESET and exception overrides
- Paged memory implementation included

## Changelog

### v0.2.1 (2026)

**Cycle Accuracy Improvements:**
- Added configurable cycle granularity via `set_cycle_granularity()` for Atari ST and similar systems
- Added wait state support via `AddressBus::wait_cycles()` method
- Fixed exception timing to match MC68000UM Table 8-14:
  - TRAP #n: 34 → 38 cycles
  - Divide by Zero: 38 → 42 cycles (+ EA)
  - CHK: 40 → 44 cycles (+ EA)

### v0.2.0 (2026)

**Modernization:**
- Updated to Rust Edition 2021
- Consolidated into single `r68k` crate (merged r68k-common into r68k)
- Fixed all Clippy warnings
- Updated dependencies to modern versions (quickcheck 1.0, rand 0.8, etc.)

**API Additions:**
- Added `reset_instruction()` method to `AddressBus` trait for RESET instruction handling (default no-op)
- Added `Cpu` type alias and `Cpu::new()` convenience constructor
- Comprehensive API documentation for crates.io publication

### v0.1.0 (2016)

- Initial port of Musashi 68000 to Rust

## Testing Philosophy

All 64k possible opcodes have been A/B-tested against Musashi using [QuickCheck](https://github.com/BurntSushi/quickcheck). There are about 54,000 valid opcodes for the m68k (the remaining 11,500 do not represent valid instructions).

Using QuickCheck means we first generate a *randomized* CPU state (including random values for all D and A registers, and the status register), then both Musashi and r68k are put in this state, the instruction under test is executed, and the resulting state is compared for any differences. All memory accesses are also compared, including address, operation size, value, and address space.

## Credits

This project is based on the original [r68k](https://github.com/marhel/r68k) by Martin Hellspong ([@marhel](https://github.com/marhel)), who ported Musashi to Rust back in 2016.

In 2026, [@slippyex](https://github.com/slippyex) modernized the codebase: updated to Rust Edition 2021, consolidated the crates, updated all dependencies, and aligned the emulation with the latest available Musashi version.

## License

MIT License - see [LICENSE.md](LICENSE.md)

## References

- [M68000 Programmer's Reference Manual](https://www.nxp.com/files/archives/doc/ref_manual/M68000PRM.pdf)
- [M68000 User's Manual](http://cache.freescale.com/files/32bit/doc/ref_manual/MC68000UM.pdf)
- [Musashi - Original C implementation](https://github.com/kstenerud/Musashi)
- [Musashi - Original Rust implementation](https://github.com/marhel/r68k)
