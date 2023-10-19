use crate::sys::gdt;
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::interrupts;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

// TODO(kosinw): Setup proper IRQ masking

macro_rules! irq_handler {
    ($handler:ident, $irq:expr) => {
        pub extern "x86-interrupt" fn $handler(
            stack_frame: x86_64::structures::idt::InterruptStackFrame,
        ) {
            let handlers = IRQ_VECTORS.lock();
            handlers[$irq](stack_frame);
            end_of_interrupt($irq);
        }
    };
}

type IrqVector = fn(InterruptStackFrame);

pub const PIC_IRQ_OFFSET: u8 = 0x20;

static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new_contiguous(PIC_IRQ_OFFSET) });

lazy_static! {
    static ref IRQ_VECTORS: Mutex<[IrqVector; 16]> = Mutex::new([|_| {}; 16]);
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        unsafe {
            idt.breakpoint.set_handler_fn(breakpoint_handler);
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt.page_fault.set_handler_fn(page_fault_handler);
        }

        idt[irq_offset_handler(0)].set_handler_fn(irq0_handler);
        idt[irq_offset_handler(1)].set_handler_fn(irq1_handler);
        idt[irq_offset_handler(2)].set_handler_fn(irq2_handler);
        idt[irq_offset_handler(3)].set_handler_fn(irq3_handler);
        idt[irq_offset_handler(4)].set_handler_fn(irq4_handler);
        idt[irq_offset_handler(5)].set_handler_fn(irq5_handler);
        idt[irq_offset_handler(6)].set_handler_fn(irq6_handler);
        idt[irq_offset_handler(7)].set_handler_fn(irq7_handler);
        idt[irq_offset_handler(8)].set_handler_fn(irq8_handler);
        idt[irq_offset_handler(9)].set_handler_fn(irq9_handler);
        idt[irq_offset_handler(10)].set_handler_fn(irq10_handler);
        idt[irq_offset_handler(11)].set_handler_fn(irq11_handler);
        idt[irq_offset_handler(12)].set_handler_fn(irq12_handler);
        idt[irq_offset_handler(13)].set_handler_fn(irq13_handler);
        idt[irq_offset_handler(14)].set_handler_fn(irq14_handler);
        idt[irq_offset_handler(15)].set_handler_fn(irq15_handler);

        idt
    };
}

irq_handler!(irq0_handler, 0);
irq_handler!(irq1_handler, 1);
irq_handler!(irq2_handler, 2);
irq_handler!(irq3_handler, 3);
irq_handler!(irq4_handler, 4);
irq_handler!(irq5_handler, 5);
irq_handler!(irq6_handler, 6);
irq_handler!(irq7_handler, 7);
irq_handler!(irq8_handler, 8);
irq_handler!(irq9_handler, 9);
irq_handler!(irq10_handler, 10);
irq_handler!(irq11_handler, 11);
irq_handler!(irq12_handler, 12);
irq_handler!(irq13_handler, 13);
irq_handler!(irq14_handler, 14);
irq_handler!(irq15_handler, 15);

pub fn init() {
    IDT.load();
    unsafe {
        let mut pics = PICS.lock();
        pics.write_masks(0, 0);
        pics.initialize();
    }
    interrupts::enable();
}

pub fn register_irq_handler(irq: u8, vector: IrqVector) {
    interrupts::without_interrupts(|| {
        let mut vecs = IRQ_VECTORS.lock();
        vecs[irq as usize] = vector;
    });
}

fn end_of_interrupt(irq: u8) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(irq + PIC_IRQ_OFFSET);
    }
}

fn irq_offset_handler(irq: u8) -> usize {
    (PIC_IRQ_OFFSET + irq) as usize
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::debug!("EXCEPTION: BREAKPOINT");
    crate::debug!("Stack Frame: {:?}", stack_frame);
    panic!();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    crate::debug!("EXCEPTION: DOUBLE FAULT");
    crate::debug!("Stack Frame: {:?}", stack_frame);
    crate::debug!("Error Code: {}", error_code);
    panic!();
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    use x86_64::instructions::hlt;

    crate::debug!("EXCEPTION: PAGE FAULT");
    crate::debug!("Accessed Address: {:?}", Cr2::read());
    crate::debug!("Error Code: {:?}", error_code);
    crate::debug!("Stack Frame {:?}", stack_frame);
    loop { hlt(); }
}
