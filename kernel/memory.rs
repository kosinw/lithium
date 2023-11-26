pub mod addr {
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
}

pub mod kalloc {
    use crate::log;
    use crate::multiboot::{InfoFlags, MultibootInfo};
    use core::ffi::CStr;
    use core::mem::size_of;

    extern "C" {
        static KERNBASE: [u64; 0];
        static KERNSTART: [u64; 0];
        static KERNSTOP: [u64; 0];
    }

    pub fn init(mbi_ptr: *const MultibootInfo) {
        let mbi = unsafe { &*mbi_ptr };

        log!("found multiboot info at {mbi_ptr:016p}");

        // Print out total amount of memory available.
        if mbi.flags.contains(InfoFlags::MEMORY) {
            // mem_lower and mem_upper are the total number of kilobytes available
            let total_memory = (mbi.mem_lower + mbi.mem_upper) << 10;
            log!("{total_memory} total bytes available",);
        } else {
            panic!("expected {:?} in multiboot info", InfoFlags::MEMORY);
        }

        // Print out name of boot loader.
        if mbi.flags.contains(InfoFlags::BOOT_LOADER_NAME) {
            let name = unsafe { CStr::from_ptr(mbi.boot_loader_name as *const i8) };
            log!(r#"bootloader name: {name:?}"#);
        }

        // Print out kernel start and stop addresses
        let kernel_base = unsafe { KERNBASE.as_ptr() as u64 };
        let kernel_start = unsafe { KERNSTART.as_ptr() as u64 } - kernel_base;
        let kernel_end = unsafe { KERNSTOP.as_ptr() as u64 } - kernel_base;

        log!(
            "[{kernel_start:#016x}-{kernel_end:#016x}]{:indent$}KERNEL",
            "",
            indent = 13
        );

        let multiboot_start = mbi_ptr as u64;
        let multiboot_end = multiboot_start + (size_of::<MultibootInfo>() as u64);

        log!(
            "[{multiboot_start:#016x}-{multiboot_end:#016x}]{:indent$}MULTIBOOT",
            "",
            indent = 13
        );

        // Memmap flag must be set.
        if !mbi.flags.contains(InfoFlags::MEM_MAP) {
            panic!("expected {:?} in multiboot info", InfoFlags::MEM_MAP);
        }

        // Print out memory areas
        for area in mbi.memory_areas() {
            // need { area.area_type } to make a copy, unaligned memory access bc of packing
            let size_mb = area.size() as f64 / (1 << 20) as f64;
            log!(
                "[{:#016x}-{:#016x}] {:>10.2}M {}",
                area.start_address(),
                area.end_address(),
                size_mb,
                { area.area_type }
            );
        }
    }
}
