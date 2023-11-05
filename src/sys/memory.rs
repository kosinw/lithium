use x86_64::{VirtAddr, PhysAddr};
use x86_64::instructions::interrupts;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable, Translate};

use crate::info;

static mut PHYS_MEM_OFFSET: u64 = 0;

pub const HEAPBASE: u64 = 0x4444_4444_0000;
pub const HEAPSTOP: u64 = HEAPBASE + 1*1024*1024 - 1;

pub(crate) fn init(boot_info: &'static bootloader::BootInfo) {
    interrupts::without_interrupts(|| {
        let mut memory_size = 0;

        info!("Starting memory subsystem...");

        unsafe { PHYS_MEM_OFFSET = boot_info.physical_memory_offset; }

        for region in boot_info.memory_map.iter() {
            let start_addr = region.range.start_addr();
            let end_addr = region.range.end_addr();
            memory_size += end_addr - start_addr;
            info!(
                "[{:#016X}-{:#016X}] {:?}",
                start_addr, end_addr, region.region_type
            );
        }

        let mut mapper = unsafe { offset_page_table() };

        info!("Entire memory region: {}KB", memory_size >> 10);

        crate::sys::alloc::init(&boot_info.memory_map, &mut mapper);
    });
}

///
/// Translates virtual addresses into physical addresses by
/// performing a page table lookup.
///
pub fn translate_addr(virt_addr: VirtAddr) -> Option<PhysAddr> {
    let mapper = unsafe { offset_page_table() };
    mapper.translate_addr(virt_addr)
}

///
/// Translates physical addresses into virtual addresses.
///
pub fn untranslate_addr(phys_addr: PhysAddr) -> Option<VirtAddr> {
    Some(VirtAddr::new(unsafe { PHYS_MEM_OFFSET } + phys_addr.as_u64()))
}

unsafe fn offset_page_table() -> OffsetPageTable<'static> {
    let (page_table_frame, _) = Cr3::read();
    let phys = page_table_frame.start_address();
    let pmo = VirtAddr::new(PHYS_MEM_OFFSET);
    let virt: VirtAddr = pmo + phys.as_u64();
    let page_table: &mut PageTable = &mut *virt.as_mut_ptr();
    OffsetPageTable::new(page_table, pmo)
}