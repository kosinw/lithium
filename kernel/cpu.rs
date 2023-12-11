use core::arch::asm;
use core::sync::atomic::{fence, Ordering};

use x86_64::instructions::interrupts;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::model_specific::GsBase;
use x86_64::registers::segmentation::{Segment, Segment64, CS, DS, ES, GS, SS};
use x86_64::structures::gdt::Descriptor;
use x86_64::structures::gdt::GlobalDescriptorTable;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

use raw_cpuid::native_cpuid::CpuIdReaderNative;
use raw_cpuid::CpuId;

/// Number of total CPUs that are currently supported.
pub const CPU_COUNT: usize = 1;

/// Size of the trap handler stack.
pub const TRAP_STACK_SIZE: usize = 4096 * 5;

// This structure should be protected by a spinlock but locks require
// access to this structure to track the level of interrupt nesting.
// Sort of a chicken-and-egg problem..
static mut CPUS: [Cpu; CPU_COUNT] = [Cpu::new(); CPU_COUNT];

// For now, we are just hard coding a large array in .bss
// to handle for the stack. Ideally we would have allocated this
// page, but again sort of a chicken-and-egg problem with the spinlocks.
static TRAP_STACK: [u8; TRAP_STACK_SIZE] = [0; TRAP_STACK_SIZE];

/// Data and provenance for CPU TSC frequency.
///
/// Since there are many ways to obtain CPU frequency (most of them relating
/// to the CPUID instruction), this data structure tracks specifically which
/// source our CPU frequency came from.
#[derive(Debug, Clone, Copy)]
pub enum CpuFrequency {
    /// Measured processor frequency from the TSC info MSR.
    CpuIdTscInfo { hz: u64 },

    /// No valid way to measure processor frequency.
    Invalid,
}

impl CpuFrequency {
    pub const fn frequency(&self) -> u64 {
        use CpuFrequency::*;

        match *self {
            CpuIdTscInfo { hz } => hz,
            Invalid => 2000000000, // we guess the value at 2GHz
        }
    }
}

/// Per-CPU data structure that holds important information such
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Cpu {
    id: usize,                  // logical identifier of core
    freq: CpuFrequency,         // frequency which timestamp counter runs at
    tss: TaskStateSegment,      // task state segment
    gdt: GlobalDescriptorTable, // global descriptor table
}

impl Cpu {
    /// Creates a new per-cpu kernel data structure.
    pub const fn new() -> Self {
        Self {
            id: 0,
            freq: CpuFrequency::Invalid,
            tss: TaskStateSegment::new(),
            gdt: GlobalDescriptorTable::new(),
        }
    }

    /// Returns the processor frequency in megahertz (MHz).
    #[inline]
    pub fn get_frequency(&self) -> u64 {
        self.freq.frequency()
    }

    // TODO(kosinw): Actually use CPUID to check if rdtsc is available on machine.
    /// Returns the timestamp of the current processor.
    #[inline]
    pub fn get_timestamp(&self) -> u64 {
        unsafe {
            let lo: u32;
            let hi: u32;
            fence(Ordering::SeqCst);
            asm!("rdtsc", out("eax") lo, out ("edx") hi);
            fence(Ordering::SeqCst);
            u64::from(hi) << 32 | u64::from(lo)
        }
    }

    /// Returns the timer ticks with 1 microsecond resolution.
    #[inline]
    pub fn get_timer_ticks(&self) -> f64 {
        (self.get_timestamp() as f64) / (self.get_frequency() as f64)
    }
}

/// Initializes per-cpu kernel data structure for a given logical core number.
///
/// Initialization of the data structure involves creating a global descriptor table
/// for the current processor where kernel code, kernel data, user code, user data,
/// and a task segment are created to be used for traps on the processor.
///
/// Other CPUID features are detected during this initialization sequence such as the
/// frequency of the TSC register.
///
/// This function must only be called once per AP and with ID 0 for the bootstrap processor.
pub fn init(id: usize) {
    assert!(id < CPU_COUNT);

    let mut tss = TaskStateSegment::new();
    let mut gdt = GlobalDescriptorTable::new();

    // Setup task state segment for a stack since we only use a
    // single trap vector to handle all interrupts.
    // TODO(kosinw): Come up with another way for multiprocessor support in the future
    // Each proecssor should have their own trap stack.
    tss.interrupt_stack_table[0] = {
        let stack_start = VirtAddr::from_ptr(TRAP_STACK.as_ptr());
        stack_start + TRAP_STACK_SIZE
    };

    unsafe {
        let cs = gdt.add_entry(Descriptor::kernel_code_segment());
        let ds = gdt.add_entry(Descriptor::kernel_data_segment());
        let ts = gdt.add_entry(Descriptor::tss_segment_unchecked(&tss));

        // Load the newly created segment descriptors into appropriate registers

        gdt.load_unsafe();
        CS::set_reg(cs);
        DS::set_reg(ds);
        ES::set_reg(ds);
        SS::set_reg(ds);
        load_tss(ts);
    }

    // Detect the frequency of the processor.
    // TODO(kosinw): Add alternate methods of detecting the frequency and provenance,
    // for now just assume that the cpu has the tschz MSR.
    let cpuid: CpuId<CpuIdReaderNative> = CpuId::new();

    let freq = cpuid
        .get_tsc_info()
        .and_then(|x| x.tsc_frequency())
        .map_or(CpuFrequency::Invalid, |v| CpuFrequency::CpuIdTscInfo {
            hz: v,
        });

    // Ensure processor interrupts are turned off.
    interrupts::disable();

    // Save the CPU information into the a global data structure.
    unsafe {
        CPUS[id] = Cpu {
            id,
            freq,
            gdt,
            tss,
        };

        // Write the pointer of this structure into GSBASE.
        let ptr = &CPUS[id] as *const Cpu;
        GsBase::write(VirtAddr::from_ptr(ptr));
    }
}

/// Gets a reference to the per-cpu data structure for the current processor.
///
/// # Safety
/// This function is potentially unsafe because it requires initializing the
/// GSBASE register during the [`crate::cpu::init`] routine. If this routine is called
/// before cpu::init, then potential invalid data will be read and this function is
/// unsafe.
pub unsafe fn current() -> &'static Cpu {
    GS::read_base().as_ptr::<Cpu>().as_ref().unwrap()
}

/// Gets the logical number of the current processor.
///
/// # Safety
/// This function is unsafe for the same reasons that [`crate::cpu::current`] is also unsafe.
pub unsafe fn id() -> usize {
    current().id
}

/// Gets a mutable reference to the per-cpu data structure for the current processor.
///
/// # Safety
/// This function is potentially unsafe because it requires initializing the
/// GSBASE register during the cpu::init routine. If this routine is called
/// before cpu::init, then potential invalid data will be read and this function is
/// unsafe.
// pub unsafe fn current_mut() -> &'static mut Cpu {
//     GS::read_base().as_mut_ptr::<Cpu>().as_mut().unwrap()
// }

/// Gets the ticks of the current processor.
///
/// # Safety
/// This function is potentially unsafe for the stame reasons that [`crate::cpu::current`] is also unsafe.
pub unsafe fn ticks() -> f64 {
    current().get_timer_ticks()
}
