pub mod framealloc {
    use crate::arch::paging::Frame;
    use crate::log;
    use crate::multiboot::{InfoFlags, MemoryArea, MemoryAreaIter, MemoryAreaType, MultibootInfo};
    use core::ffi::CStr;
    use core::mem::size_of;

    trait FrameAllocator {
        fn allocate_frame(&mut self) -> Option<Frame>;
        fn deallocate_frame(&mut self, frame: Frame);
    }

    // TODO(kosinw): Replace this with a better allocator
    #[derive(Debug, Clone)]
    struct AreaFrameAllocator {
        next_free_frame: Frame,
        current_area: Option<&'static MemoryArea>,
        areas: MemoryAreaIter,
        kernel_start: Frame,
        kernel_end: Frame,
        multiboot_start: Frame,
        multiboot_end: Frame,
    }

    impl AreaFrameAllocator {
        pub fn new(
            kernel_start: u64,
            kernel_end: u64,
            multiboot_start: u64,
            multiboot_end: u64,
            memory_areas: MemoryAreaIter,
        ) -> AreaFrameAllocator {
            let mut allocator = AreaFrameAllocator {
                next_free_frame: Frame::containing_address(0),
                current_area: None,
                areas: memory_areas,
                kernel_start: Frame::containing_address(kernel_start),
                kernel_end: Frame::containing_address(kernel_end),
                multiboot_start: Frame::containing_address(multiboot_start),
                multiboot_end: Frame::containing_address(multiboot_end),
            };
            allocator.choose_next_area();
            allocator
        }

        fn choose_next_area(&mut self) {
            self.current_area = self
                .areas
                .clone()
                .filter(|area| matches!(area.area_type, MemoryAreaType::Available))
                .filter(|area| {
                    let address = area.end_address();
                    Frame::containing_address(address) >= self.next_free_frame // choose next frame greater than current next_free_frame
                })
                .min_by_key(|area| area.start_address());

            if let Some(area) = self.current_area {
                let start_frame = Frame::containing_address(area.addr);
                if self.next_free_frame < start_frame {
                    self.next_free_frame = start_frame;
                }
            }
        }
    }

    impl FrameAllocator for AreaFrameAllocator {
        fn allocate_frame(&mut self) -> Option<Frame> {
            if let Some(area) = self.current_area {
                // Clone the next free frame to return.
                let frame = self.next_free_frame.clone();

                // Get the last frame of the current area.
                let current_area_last_frame = {
                    let address = area.addr + area.len - 1;
                    Frame::containing_address(address)
                };

                if frame > current_area_last_frame {
                    // All the frames in the current area are used up, switch to the next one
                    self.choose_next_area();
                } else if frame >= self.kernel_start && frame < self.kernel_end {
                    // Frame is being used by kernel, skip it for next frame.
                    // Kernel must be page aligned so this should get the next frame.
                    self.next_free_frame = self.kernel_end.clone();
                } else if frame >= self.multiboot_start && frame < self.multiboot_end {
                    // Frame is being used by the multiboot info structure.
                    // Multiboot is not necessarily page aligned so this should skip to the next frame.
                    self.next_free_frame = self.multiboot_end.next_frame();
                } else {
                    // Frame is unused, just increment;
                    self.next_free_frame = self.next_free_frame.next_frame();
                    return Some(frame);
                }

                // Frame was not in a valid spot so try again
                self.allocate_frame()
            } else {
                None // no frames left
            }
        }

        fn deallocate_frame(&mut self, frame: Frame) {
            todo!()
        }
    }

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
                area.end_address() + 1,
                size_mb,
                { area.area_type }
            );
        }

        // Create allocator finally
        let mut allocator = AreaFrameAllocator::new(
            kernel_start,
            kernel_end,
            multiboot_start,
            multiboot_end,
            mbi.memory_areas(),
        );

        for i in 0.. {
            if let None = allocator.allocate_frame() {
                log!("allocated {} frames", i);
                break;
            }
        }
    }
}
