use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use linked_list_allocator::LockedHeap;
use x86_64::{
    instructions::interrupts,
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use super::memory::{HEAPBASE, HEAPSTOP};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next_frame: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next_frame: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        self.memory_map
            .iter()
            .filter(|r| r.region_type == MemoryRegionType::Usable)
            .map(|r| r.range.start_addr()..r.range.end_addr())
            .map(|r| r.step_by(0x1000))
            .flatten()
            .map(|a| PhysFrame::containing_address(PhysAddr::new(a)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next_frame);
        self.next_frame += 1;
        frame
    }
}

fn init_heap(mapper: &mut impl Mapper<Size4KiB>, allocator: &mut impl FrameAllocator<Size4KiB>) {
    let page_range = {
        let page_start = Page::containing_address(VirtAddr::new(HEAPBASE));
        let page_end = Page::containing_address(VirtAddr::new(HEAPSTOP));
        Page::range_inclusive(page_start, page_end)
    };

    for page in page_range {
        let frame = allocator
            .allocate_frame()
            .expect("Could not initialize heap");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            let _ = mapper
                .map_to(page, frame, flags, allocator)
                .expect("Could not intiialize heap")
                .flush();
        }
    }

    unsafe {
        ALLOCATOR.lock().init(HEAPBASE as *mut u8, (HEAPSTOP - HEAPBASE + 1) as usize);
    }
}

pub(crate) fn init(memory_map: &'static MemoryMap, mapper: &mut impl Mapper<Size4KiB>) {
    interrupts::without_interrupts(|| {
        let mut boot_allocator = unsafe { BootInfoFrameAllocator::init(memory_map) };
        init_heap(mapper, &mut boot_allocator);
    });
}
