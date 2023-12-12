use x86_64::set_general_handler;
use x86_64::structures::idt::InterruptStackFrame;

use crate::cpu;

/// Handles traps (interrupts, nmi, exceptions, etc.) raised in kernel space.
fn kerneltrap(_stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    crate::log!("cause={index} code={error_code:?}");
}

/// Initializes the trap handling mechanism for the kernel.
///
/// This function sets up the necessary structures and configurations
/// for handling traps and exceptions within the Lithium unikernel. Traps, which include
/// exceptions, interrupts, and other asynchronous events, are essential for the correct
/// operation of the kernel.
pub fn init() {
    use x86_64::instructions::tables::sidt;
    unsafe {
        let cpu = unsafe { cpu::current_mut() };
        set_general_handler!(&mut cpu.idt, kerneltrap);

        crate::log!(
            "trap::init(): previous IDT is located at {:016p}",
            sidt().base.as_ptr::<u8>()
        );

        cpu.idt.load();

        crate::log!(
            "trap::init(): current IDT is located at {:016p}",
            sidt().base.as_ptr::<u8>()
        );
    }
}
