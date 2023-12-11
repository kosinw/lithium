use crate::log;
use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size4KiB;
use x86_64::VirtAddr;

// TODO(kosinw): Replace this with a custom buddy allocator (debugging is too hard rn...)
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initializes the heap for the kernel.
///
/// This function is responsible for setting up the heap memory for dynamic memory allocation
/// within the kernel. It configures the allocator, allocates an initial heap region, and
/// performs any necessary setup for the memory management subsystem.
pub fn init() {
    use crate::memory;
    use crate::memory::HEAP_SIZE;
    use crate::memory::HEAP_START;

    log!("heap::init(): allocating physical region for heap...");

    let va = VirtAddr::new(HEAP_START);
    let region = unsafe {
        memory::allocate_physical_region(HEAP_SIZE as usize)
            .expect("could not allocate enough physical space for heap")
    };
    let pa = region.start_address();
    let size = region.size();

    log!("heap::init(): allocating physical region for heap... [ \x1b[0;32mOK\x1b[0m ]");
    log!("heap::init(): using phys region [{:#016x}-{:#016x}]", region.start_address().as_u64(), region.end_address().as_u64());
    log!("heap::init(): using virt region [{:#016x}-{:#016x}]", HEAP_START, HEAP_START + size as u64);

    assert!(
        size >= HEAP_SIZE as usize,
        "heap region returned by physical allocator is too small"
    );

    unsafe {
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        memory::map_pages::<Size4KiB>(va, pa, size as u64, flags)
            .expect("failed to map heap pages");
    }

    // Tell allocator about new heap region.
    unsafe {
        ALLOCATOR.lock().init(va.as_mut_ptr(), size);
    }

    log!("heap::init(): successfully initialized [ \x1b[0;32mOK\x1b[0m ]");

}
