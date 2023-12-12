use x86_64::instructions::interrupts;
use x86_64::instructions::port::PortWriteOnly;
use x86_64::set_general_handler;
use x86_64::structures::idt::ExceptionVector;
use x86_64::structures::idt::InterruptStackFrame;

use crate::console;
use crate::cpu;
use crate::log;

const IO_PIC1_COMMAND: u16 = 0x20;
const IO_PIC1_DATA: u16 = 0x21;
const IO_PIC2_COMMAND: u16 = 0xA0;
const IO_PIC2_DATA: u16 = 0xA1;

pub const TRAP_IRQ0: u8 = 0x20;
pub const IRQ_SLAVE: u8 = 2;
pub const IRQ_COM1: u8 = 4;

const CMD_END_OF_INTERRUPT: u8 = 0x20;

/// Handles traps (interrupts, nmi, exceptions, etc.) raised in kernel space.
fn kerneltrap(_stack_frame: InterruptStackFrame, index: u8, _error_code: Option<u64>) {
    // log!("trap::kerneltrap(): hello from trap handler!");
    match index {
        x if x == ExceptionVector::GeneralProtection as u8 => {
            panic!("trap::kerneltrap(): general protection fault")
        }
        x if x == ExceptionVector::Page as u8 => panic!("trap::kerneltrap(): page fault"),
        x if x == (IRQ_COM1 + TRAP_IRQ0) => {
            console::interrupt();
            end_of_interrupt(x);
        }
        _ => panic!("trap::kerneltrap(): unknown trap kind {}", index),
    }
}

bitflags::bitflags! {
    // ICW1 flags
    struct ICW1: u8 {
        const ICW4 = 0x01;      /* Indicates that ICW4 will be present */
        const SINGLE = 0x02;    /* Single (no cascade) mode */
        const INTERVAL4 = 0x04; /* Call address interval 4 (8) */
        const LEVEL = 0x08;     /* Level triggered (edge) mode */
        const INIT = 0x10;      /* Initialization - required! */
    }

    // ICW4 flags
    struct ICW4: u8 {
        const MODE_8086 = 0x01;     /* 8086/88 (MCS-80/85) mode */
        const AUTO_EOI = 0x02;      /* Auto (normal) EOI */
        const BUF_SLAVE = 0x08;     /* Buffered mode/slave */
        const BUF_MASTER = 0x0C;    /* Buffered mode/master */
        const SFNM = 0x10;          /* Special fully nested (not) */
    }
}

/// Acknowledge end of interrupt for PIC device.
fn end_of_interrupt(v: u8) {
    if (TRAP_IRQ0..TRAP_IRQ0 + 8).contains(&v) {
        let mut command_port = PortWriteOnly::new(IO_PIC1_COMMAND);
        unsafe {
            command_port.write(CMD_END_OF_INTERRUPT);
        }
    } else if (TRAP_IRQ0 + 8..TRAP_IRQ0 + 16).contains(&v) {
        let mut command_port = PortWriteOnly::new(IO_PIC2_COMMAND);
        unsafe {
            command_port.write(CMD_END_OF_INTERRUPT);
        }
    }
}

/// Sets the IRQ enable mask.
fn set_irq_mask(mask: u16) {
    unsafe {
        let mut master_data_port = PortWriteOnly::new(IO_PIC1_DATA);
        let mut slave_data_port = PortWriteOnly::new(IO_PIC2_DATA);

        let cpu = cpu::current_mut();
        cpu.irq_mask = mask;
        master_data_port.write((mask & 0xff) as u8);
        slave_data_port.write((mask >> 8) as u8);
    }
}

/// Enables the IRQ.
pub fn enable_irq(irq: u8) {
    let cpu = unsafe { cpu::current() };
    set_irq_mask(cpu.irq_mask & !(1 << irq));
}

/// Initializes the PIC8259A interrupt controller.
fn enable_pic8259a() {
    unsafe {
        let mut master_data_port = PortWriteOnly::new(IO_PIC1_DATA);
        let mut master_command_port = PortWriteOnly::new(IO_PIC1_COMMAND);
        let mut slave_data_port = PortWriteOnly::new(IO_PIC2_DATA);
        let mut slave_command_port = PortWriteOnly::new(IO_PIC2_COMMAND);

        // Setup all interrupts but IRQ slave line to be masked.
        let cpu = cpu::current_mut();
        cpu.irq_mask = !(1 << IRQ_SLAVE);

        // Mask all interrupts.
        master_data_port.write(0xFFu8);
        slave_data_port.write(0xFFu8);

        // Initialize master PIC.
        // ICW1: edge triggering, cascaded mode
        master_command_port.write((ICW1::ICW4 | ICW1::INIT).bits());
        // ICW2: vector offset
        master_data_port.write(TRAP_IRQ0);
        // ICW3: bit mask of IRQ lines connected to slave
        master_data_port.write(1 << IRQ_SLAVE);
        // ICW4: some other configuration stuff
        master_data_port.write(ICW4::MODE_8086.bits());

        // Initialize slave PIC.
        // ICW1: edge triggering, cascaded modes
        slave_command_port.write((ICW1::ICW4 | ICW1::INIT).bits());
        // ICW2: vector offset
        slave_data_port.write(TRAP_IRQ0 + 0x8u8);
        // ICW3: which master line are we connected to?
        slave_data_port.write(IRQ_SLAVE);
        // ICW4: some other configuration stuff
        slave_data_port.write(ICW4::MODE_8086.bits());

        // Enable console interrupts.
        console::enable_interrupts();
    }
}

/// Initializes the trap handling mechanism for the kernel.
///
/// This function sets up the necessary structures and configurations
/// for handling traps and exceptions within the Lithium unikernel. Traps, which include
/// exceptions, interrupts, and other asynchronous events, are essential for the correct
/// operation of the kernel.
pub fn init() {
    // First we set up our general purpose kernel trap handler.
    use x86_64::instructions::tables::sidt;
    let cpu = unsafe { cpu::current_mut() };
    set_general_handler!(&mut cpu.idt, kerneltrap);

    log!(
        "trap::init(): previous IDT is located at {:016p}",
        sidt().base.as_ptr::<u8>()
    );

    cpu.idt.load();

    log!(
        "trap::init(): current IDT is located at {:016p}",
        sidt().base.as_ptr::<u8>()
    );

    // Enable legacy PIC device.
    enable_pic8259a();

    // Finally enable interrupts.
    interrupts::enable();

    log!("trap::init(): interrupts are now enabled [ \x1b[0;32mOK\x1b[0m ]");
}
