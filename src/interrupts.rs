//! Interrupt handling for the 68000 CPU.
//!
//! This module provides the [`InterruptController`] trait for implementing
//! custom interrupt handling, along with [`AutoInterruptController`] which
//! provides standard autovectored interrupt support.
//!
//! # 68000 Interrupt System
//!
//! The 68000 supports 7 interrupt priority levels (1-7), where level 7 is
//! non-maskable (NMI). Interrupts are only processed if their priority
//! exceeds the current interrupt mask in the status register.
//!
//! When an interrupt is acknowledged, the controller provides a vector number
//! that determines which exception handler is called.

/// Trait for implementing custom interrupt controllers.
///
/// Implement this trait to provide custom interrupt handling logic,
/// such as vectored interrupts from specific hardware.
///
/// # Example
///
/// ```rust
/// use r68k::interrupts::InterruptController;
///
/// struct MyInterruptController {
///     pending: u8,
///     vectors: [u8; 8],
/// }
///
/// impl InterruptController for MyInterruptController {
///     fn reset_external_devices(&mut self) {
///         self.pending = 0;
///     }
///
///     fn highest_priority(&self) -> u8 {
///         (8 - self.pending.leading_zeros()) as u8
///     }
///
///     fn acknowledge_interrupt(&mut self, priority: u8) -> Option<u8> {
///         self.pending &= !(1 << (priority - 1));
///         Some(self.vectors[priority as usize])
///     }
/// }
/// ```
pub trait InterruptController
{
    /// Called when a RESET instruction is executed.
    ///
    /// Reset any pending interrupt state.
    fn reset_external_devices(&mut self);

    /// Returns the highest pending interrupt priority level (1-7), or 0 if none.
    fn highest_priority(&self) -> u8;

    /// Acknowledge an interrupt and return its vector number.
    ///
    /// Called when the CPU begins processing an interrupt. The controller
    /// should clear the pending interrupt and return the exception vector.
    ///
    /// Return `None` to signal a spurious interrupt.
    fn acknowledge_interrupt(&mut self, priority: u8) -> Option<u8>;
}

/// Vector number for spurious interrupts (no vector provided).
pub const SPURIOUS_INTERRUPT: u8 = 0x18;
const AUTOVECTOR_BASE: u8 = 0x18;

/// Standard autovectored interrupt controller.
///
/// This controller provides the standard 68000 autovectored interrupt behavior,
/// where each priority level maps to a fixed vector (24-30 for levels 1-7).
///
/// # Example
///
/// ```rust
/// use r68k::interrupts::{AutoInterruptController, InterruptController};
///
/// let mut ctrl = AutoInterruptController::new();
///
/// // Request an interrupt at level 5
/// ctrl.request_interrupt(5);
///
/// // Check highest pending priority
/// assert_eq!(ctrl.highest_priority(), 5);
///
/// // Acknowledge clears the interrupt and returns vector 29
/// assert_eq!(ctrl.acknowledge_interrupt(5), Some(29));
/// assert_eq!(ctrl.highest_priority(), 0);
/// ```
#[derive(Default)]
pub struct AutoInterruptController {
    level: u8
}

impl AutoInterruptController {
    /// Creates a new autovectored interrupt controller with no pending interrupts.
    pub fn new() -> AutoInterruptController {
        AutoInterruptController { level: 0 }
    }

    /// Requests an interrupt at the given priority level (1-7).
    ///
    /// Multiple interrupt levels can be pending simultaneously.
    /// The CPU will process the highest priority pending interrupt
    /// that exceeds its current interrupt mask.
    ///
    /// # Panics
    ///
    /// Panics if `irq` is not in the range 1-7.
    pub fn request_interrupt(&mut self, irq: u8) -> u8
    {
        assert!(irq > 0 && irq < 8);
        self.level |= 1 << (irq - 1);
        self.level
    }
}

impl InterruptController for AutoInterruptController {
    fn reset_external_devices(&mut self)
    {
        self.level = 0;
    }

    fn highest_priority(&self) -> u8 {
        (8 - self.level.leading_zeros()) as u8
    }

    fn acknowledge_interrupt(&mut self, priority: u8) -> Option<u8> {
        self.level &= !(1 << (priority - 1));
        Some(AUTOVECTOR_BASE + priority)
    }
}


#[cfg(test)]
mod tests {
    use super::{InterruptController, AutoInterruptController,
        AUTOVECTOR_BASE};

    #[test]
    fn keeps_track_of_priority() {
        let mut ctrl = AutoInterruptController { level: 0 };
        ctrl.request_interrupt(2);
        ctrl.request_interrupt(5);
        assert_eq!(5, ctrl.highest_priority());
    }
    #[test]
    fn auto_resets_on_ack() {
        let mut ctrl = AutoInterruptController { level: 0 };
        ctrl.request_interrupt(2);
        ctrl.request_interrupt(5);
        assert_eq!(Some(AUTOVECTOR_BASE + 5), ctrl.acknowledge_interrupt(5));
        assert_eq!(2, ctrl.highest_priority());
    }
    #[test]
    fn resets_irq_level_on_external_device_reset() {
        let mut ctrl = AutoInterruptController { level: 0 };
        ctrl.request_interrupt(2);
        ctrl.request_interrupt(5);
        ctrl.reset_external_devices();
        assert_eq!(0, ctrl.highest_priority());
    }
}