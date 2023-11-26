#![allow(dead_code)]

pub mod asm {
    use super::segmentation::{DescriptorTablePointer, SegmentSelector};
    use bitflags::bitflags;
    use core::arch::asm;

    bitflags! {
        pub struct RFlags: u64 {
            const ID = 1 << 21;
            const VIRTUAL_INTERRUPT_PENDING = 1 << 20;
            const VIRTUAL_INTERRUPT = 1 << 19;
            const ALIGNMENT_CHECK = 1 << 18;
            const VIRTUAL_8086_MODE = 1 << 17;
            const RESUME_FLAG = 1 << 16;
            const NESTED_TASK = 1 << 14;
            const IOPL_HIGH = 1 << 13;
            const IOPL_LOW = 1 << 12;
            const OVERFLOW_FLAG = 1 << 11;
            const DIRECTION_FLAG = 1 << 10;
            const INTERRUPT_FLAG = 1 << 9;
            const TRAP_FLAG = 1 << 8;
            const SIGN_FLAG = 1 << 7;
            const ZERO_FLAG = 1 << 6;
            const AUXILIARY_CARRY_FLAG = 1 << 4;
            const PARITY_FLAG = 1 << 2;
            const CARRY_FLAG = 1;
        }
    }

    pub unsafe fn r_flags() -> RFlags {
        let raw: u64;
        asm!("pushfq; pop {}", out(reg) raw, options(preserves_flags));
        RFlags::from_bits_truncate(raw)
    }

    /// Writes to the RFLAGS register, preserves already set bits.
    pub unsafe fn w_flags(flags: RFlags) {
        let old = r_flags().bits();
        let reserved = old & !(RFlags::all().bits());
        let raw = reserved | flags.bits();
        asm!("push {}; popfq", in (reg) raw, options(preserves_flags));
    }

    pub unsafe fn sti() {
        asm!("sti");
    }

    pub unsafe fn cli() {
        asm!("cli");
    }

    pub unsafe fn pause() {
        asm!("pause");
    }

    pub unsafe fn hlt() {
        asm!("hlt");
    }

    pub unsafe fn is_interrupt_enabled() -> bool {
        r_flags().contains(RFlags::INTERRUPT_FLAG)
    }

    pub unsafe fn w_gsbase(w: u64) {
        asm!("wrgsbase {}", in(reg) w);
    }

    pub unsafe fn r_msr(index: u32) -> u64 {
        let lo: u32;
        let hi: u32;
        asm!("rdmsr", in("ecx") index, out("edx") hi, out("eax") lo);
        (u64::from(hi) << 2) | u64::from(lo)
    }

    // TODO(kosinw): Fix me and actually properly get TSC frequency
    pub unsafe fn r_tschz() -> u64 {
        2_000_000_000
    }

    pub unsafe fn r_tsc() -> u128 {
        let lo: u32;
        let hi: u32;
        asm!("rdtsc", out("eax") lo, out ("edx") hi);
        u128::from(hi) << 32 | u128::from(lo)
    }

    pub unsafe fn w_codeseg(selector: &SegmentSelector) {
        asm!(
            "push {selector}",
            "lea {tmp}, [1f + rip]",
            "push {tmp}",
            "retfq",
            "1:",
            selector = in(reg) u64::from(selector.0),
            tmp = lateout(reg) _
        );
    }

    pub unsafe fn w_dataseg(selector: &SegmentSelector) {
        asm!("mov ds, {0:x}", in(reg) selector.0, options(preserves_flags));
    }

    pub unsafe fn w_taskseg(selector: &SegmentSelector) {
        asm!("ltr {0:x}", in(reg) selector.0, options(preserves_flags));
    }

    pub unsafe fn lgdt(gdt: &DescriptorTablePointer) {
        asm!("lgdt [{}]", in (reg) gdt, options(preserves_flags));
    }

    pub unsafe fn xchg(word: &mut u64, mut value: u64) -> u64 {
        asm!("lock xchg [{0}], {1}", in(reg) word, inout(reg) value, options(nostack));
        value
    }

    pub unsafe fn outb(port: u16, data: u8) {
        asm!("out dx, al", in("dx") port, in("al") data);
    }

    pub unsafe fn inb(port: u16) -> u8 {
        let r: u8;
        asm!("in al, dx", out("al") r, in("dx") port);
        r
    }
}

pub mod segmentation {
    use bit_field::BitField;
    use bitflags::bitflags;
    use core::mem::size_of;

    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed(4))]
    pub struct TaskStateSegment {
        _reserved0: u32,
        pub privilege_stack_table: [u64; 3],
        _reserved1: u64,
        pub interrupt_stack_table: [u64; 7],
        _reserved2: u64,
        _reserved3: u16,
        pub iomap_base: u16,
    }

    impl TaskStateSegment {
        pub const fn new() -> TaskStateSegment {
            TaskStateSegment {
                privilege_stack_table: [0u64; 3],
                interrupt_stack_table: [0u64; 7],
                iomap_base: size_of::<TaskStateSegment>() as u16,
                _reserved0: 0,
                _reserved1: 0,
                _reserved2: 0,
                _reserved3: 0,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed(2))]
    pub struct DescriptorTablePointer {
        pub size: u16,
        pub base: u64,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct GlobalDescriptorTable {
        table: [u64; 8],
        len: usize,
    }

    impl GlobalDescriptorTable {
        pub const fn new() -> GlobalDescriptorTable {
            GlobalDescriptorTable {
                table: [0u64; 8],
                len: 1,
            }
        }

        pub fn add_entry(&mut self, entry: SegmentDescriptor) -> SegmentSelector {
            let index = match entry {
                SegmentDescriptor::UserSegment(value) => {
                    if self.len > self.table.len().saturating_sub(1) {
                        // too many items in GDT
                        panic!("too many entries in GDT");
                    }
                    self.push(value)
                }
                SegmentDescriptor::SystemSegment(val_low, val_hi) => {
                    if self.len > self.table.len().saturating_sub(2) {
                        // too many items in GDT
                        panic!("too many entries in GDT");
                    }
                    let index = self.push(val_low);
                    self.push(val_hi);
                    index
                }
            };
            SegmentSelector::new(index as u16, entry.dpl())
        }

        pub fn load(&self) {
            unsafe {
                super::asm::lgdt(&self.pointer());
            }
        }

        fn pointer(&self) -> DescriptorTablePointer {
            DescriptorTablePointer {
                size: (self.len * size_of::<u64>() - 1) as u16,
                base: self.table.as_ptr() as u64,
            }
        }

        fn push(&mut self, value: u64) -> usize {
            let index = self.len;
            self.table[index] = value;
            self.len += 1;
            index
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct SegmentSelector(pub u16);

    impl SegmentSelector {
        pub const fn new(index: u16, rpl: u16) -> SegmentSelector {
            SegmentSelector(index << 3 | rpl)
        }

        pub const NULL: Self = Self::new(0, 0);

        pub fn index(self) -> u16 {
            self.0 >> 3
        }

        pub fn rpl(self) -> u16 {
            self.0.get_bits(0..2)
        }

        pub fn set_rpl(&mut self, rpl: u16) {
            self.0.set_bits(0..2, rpl);
        }
    }

    bitflags! {
        /// Flags for a GDT descriptor. Not all flags are valid for all descriptor types.
        #[derive(Debug, Clone, Copy)]
        pub struct SegmentDescriptorFlags: u64 {
            const ACCESSED          = 1 << 40;
            const WRITABLE          = 1 << 41;
            const CONFORMING        = 1 << 42;
            const EXECUTABLE        = 1 << 43;
            const USER_SEGMENT      = 1 << 44;
            const DPL_RING_3        = 3 << 45;
            const PRESENT           = 1 << 47;
            const AVAILABLE         = 1 << 52;
            const LONG_MODE         = 1 << 53;
            const DEFAULT_SIZE      = 1 << 54;
            const GRANULARITY       = 1 << 55;
            const LIMIT_0_15        = 0xFFFF;
            const LIMIT_16_19       = 0xF << 48;
            const BASE_0_23         = 0xFF_FFFF << 16;
            const BASE_24_31        = 0xFF << 56;
        }
    }

    impl SegmentDescriptorFlags {
        const COMMON: Self = Self::from_bits_truncate(
            Self::USER_SEGMENT.bits()
                | Self::PRESENT.bits()
                | Self::WRITABLE.bits()
                | Self::ACCESSED.bits()
                | Self::LIMIT_0_15.bits()
                | Self::LIMIT_16_19.bits()
                | Self::GRANULARITY.bits(),
        );

        pub const KERNEL_DATA: Self =
            (Self::from_bits_truncate(Self::COMMON.bits() | Self::DEFAULT_SIZE.bits()));

        pub const KERNEL_CODE64: Self = Self::from_bits_truncate(
            Self::COMMON.bits() | Self::EXECUTABLE.bits() | Self::LONG_MODE.bits(),
        );

        pub const USER_DATA: Self =
            Self::from_bits_truncate(Self::KERNEL_DATA.bits() | Self::DPL_RING_3.bits());

        pub const USER_CODE64: Self =
            Self::from_bits_truncate(Self::KERNEL_CODE64.bits() | Self::DPL_RING_3.bits());
    }

    /// Segmentation is not supported in 64-bit mode, so most of the descriptor
    /// contents are ignored.
    pub enum SegmentDescriptor {
        UserSegment(u64),
        SystemSegment(u64, u64),
    }

    impl SegmentDescriptor {
        pub const fn dpl(self) -> u16 {
            let value_low = match self {
                SegmentDescriptor::UserSegment(v) => v,
                SegmentDescriptor::SystemSegment(v, _) => v,
            };
            let dpl = (value_low & SegmentDescriptorFlags::DPL_RING_3.bits()) >> 45;
            dpl as u16
        }

        pub const fn kernel_code_segment() -> SegmentDescriptor {
            SegmentDescriptor::UserSegment(SegmentDescriptorFlags::KERNEL_CODE64.bits())
        }

        pub const fn kernel_data_segment() -> SegmentDescriptor {
            SegmentDescriptor::UserSegment(SegmentDescriptorFlags::KERNEL_DATA.bits())
        }

        pub const fn user_data_segment() -> SegmentDescriptor {
            SegmentDescriptor::UserSegment(SegmentDescriptorFlags::USER_DATA.bits())
        }

        pub const fn user_code_segment() -> SegmentDescriptor {
            SegmentDescriptor::UserSegment(SegmentDescriptorFlags::USER_CODE64.bits())
        }

        pub fn tss_segment(tss: &TaskStateSegment) -> SegmentDescriptor {
            use self::SegmentDescriptorFlags as Flags;

            let ptr = (tss as *const TaskStateSegment) as u64;

            let mut low = Flags::PRESENT.bits();

            // base
            low.set_bits(16..40, ptr.get_bits(0..24));
            low.set_bits(56..64, ptr.get_bits(24..32));
            // limit (the `-1` in needed since the bound is inclusive)
            low.set_bits(0..16, (size_of::<TaskStateSegment>() - 1) as u64);
            // type (0b1001 = available 64-bit tss)
            low.set_bits(40..44, 0b1001);

            let mut high = 0;
            high.set_bits(0..32, ptr.get_bits(32..64));

            SegmentDescriptor::SystemSegment(low, high)
        }
    }
}

pub mod paging {
    use core::fmt;

    pub const PAGESIZE: usize = 4096;

    #[derive(Debug, Clone, Copy)]
    #[repr(C, align(4096))]
    pub struct Page([u8; PAGESIZE]);

    impl Page {
        pub const fn empty() -> Page {
            Page([0; PAGESIZE])
        }

        pub fn as_ptr_mut(&mut self) -> *mut Page {
            self as *mut Page
        }

        pub fn as_ptr(&self) -> *const Page {
            self as *const Page
        }
    }

    /// Align address downwards.
    ///
    /// Returns the greatest `x` with alignment `align` so that `x <= addr`.
    ///
    /// Panics if the alignment is not a power of two.
    #[inline]
    pub const fn align_down(addr: u64, align: u64) -> u64 {
        assert!(align.is_power_of_two(), "`align` must be a power of two");
        addr & !(align - 1)
    }

    /// Align address upwards.
    ///
    /// Returns the smallest `x` with alignment `align` so that `x >= addr`.
    ///
    /// Panics if the alignment is not a power of two or if an overflow occurs.
    #[inline]
    pub const fn align_up(addr: u64, align: u64) -> u64 {
        assert!(align.is_power_of_two(), "`align` must be a power of two");
        let align_mask = align - 1;
        if addr & align_mask == 0 {
            addr // already aligned
        } else {
            // FIXME: Replace with .expect, once `Option::expect` is const.
            if let Some(aligned) = (addr | align_mask).checked_add(1) {
                aligned
            } else {
                panic!("attempt to add with overflow")
            }
        }
    }

    /// A 4KiB physical memory frame.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(C)]
    pub struct Frame {
        start_address: u64,
    }

    impl fmt::Debug for Frame {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_fmt(format_args!(
                "Frame[{}]({:#x})",
                "4KiB",
                self.start_address
            ))
        }
    }

    impl Frame {
        pub fn containing_address(address: u64) -> Frame {
            Frame {
                start_address: align_down(address, PAGESIZE as u64),
            }
        }

        pub const fn start_address(&self) -> u64 {
            self.start_address
        }

        pub const fn size(&self) -> u64 {
            return PAGESIZE
        }

        pub fn next_frame(&self) -> Frame {
            Frame {
                start_address: align_down(self.start_address + PAGESIZE as u64, PAGESIZE as u64),
            }
        }
    }
}

pub mod cpu {
    use super::asm::{self, is_interrupt_enabled};
    use super::paging::Page;
    use super::segmentation::{GlobalDescriptorTable, SegmentDescriptor, TaskStateSegment};
    use core::arch::asm;

    #[derive(Debug, Clone, Copy)]
    #[repr(C, align(4096))]
    pub struct Cpu {
        self_ptr: *mut Cpu,         // pointer to this structure
        id: u32,                    // kernel-assigned identifier of core
        clock_freq: u64,            // frequency which clock runs at
        noff: u32,                  // depth of push_off() nesting.
        intena: bool,               // were interrupts enabled before push_off()?
        tss: TaskStateSegment,      // task state segment
        gdt: GlobalDescriptorTable, // global descriptor table
    }

    pub unsafe fn push_interrupt_off() {
        let cpu = current_mut();
        let enabled = is_interrupt_enabled();

        asm::cli();

        if cpu.noff == 0 {
            cpu.intena = enabled;
        }

        cpu.noff += 1;
    }

    pub unsafe fn pop_interrupt_off() {
        let cpu = current_mut();
        let enabled = is_interrupt_enabled();

        assert!(!enabled, "pop_intr(): interrupts should not be enabled");
        assert!(cpu.noff != 0, "pop_intr(): noff = 0");

        cpu.noff -= 1;

        if cpu.noff == 0 && cpu.intena {
            asm::sti();
        }
    }

    pub unsafe fn init(page: &mut Page, id: u32) {
        let cpu = &mut *(page.as_ptr_mut() as *mut Cpu);
        let mut tss = TaskStateSegment::new();
        let mut gdt = GlobalDescriptorTable::new();

        // Setup task state segment for a double fault handler stack.
        tss.interrupt_stack_table[0] = {
            let stack_start = page.as_ptr() as u64;
            let stack_end = stack_start + 4096;
            stack_end
        };

        let cs = gdt.add_entry(SegmentDescriptor::kernel_code_segment());
        let ds = gdt.add_entry(SegmentDescriptor::kernel_data_segment());
        gdt.add_entry(SegmentDescriptor::user_code_segment());
        gdt.add_entry(SegmentDescriptor::user_data_segment());
        let ts = gdt.add_entry(SegmentDescriptor::tss_segment(&tss));

        *cpu = Cpu {
            self_ptr: cpu,
            clock_freq: asm::r_tschz(),
            noff: 0,
            intena: false,
            tss,
            gdt,
            id,
        };

        gdt.load();

        asm::w_codeseg(&cs);
        asm::w_dataseg(&ds);
        asm::w_taskseg(&ts);
        asm::w_gsbase(cpu as *mut Cpu as u64);
    }

    pub fn id() -> u32 {
        current().id
    }

    pub fn current() -> &'static Cpu {
        unsafe {
            use core::mem::transmute;
            let base: u64;
            asm!("mov {o}, gs:0", o = out(reg) base, options(preserves_flags));
            transmute::<u64, &Cpu>(base)
        }
    }

    pub fn current_mut() -> &'static mut Cpu {
        unsafe {
            use core::mem::transmute;
            let base: u64;
            asm!("mov {o}, gs:0", o = out(reg) base, options(preserves_flags));
            transmute::<u64, &mut Cpu>(base)
        }
    }
}

#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    // if interrupts are disabled, disable them now
    unsafe {
        cpu::push_interrupt_off();
    }

    // do `f` while interrupts are disabled
    let ret = f();

    // re-enable interrupts if they were previously enabled
    unsafe {
        cpu::pop_interrupt_off();
    }

    // return the result of `f` to the caller
    ret
}
