use crate::log;
use crate::multiboot::InfoFlags;
use crate::multiboot::MemoryAreaType;
use crate::multiboot::MultibootInformation;
use crate::println;
use spin::Mutex;
use x86_64::structures::paging::page::AddressNotAligned;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::FrameDeallocator;
use x86_64::structures::paging::PageSize;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PhysFrame;
use x86_64::{PhysAddr, VirtAddr};

/// Maximum number of physical memory regions that can be used by physical allocator.
const MAX_PHYS_REGIONS: usize = 16;

/// Physical frame allocator. Responsible for allocating physical frames for virtual memory manager.
static mut FRAME_ALLOCATOR: Mutex<PhysicalAllocator> = Mutex::new(PhysicalAllocator::new());

// Kernel page table.
static mut KERNEL_PAGETABLE: Mutex<PageTable> = Mutex::new(PageTable::new());

/// Represents a physical memory frame.
///
/// A `Frame` struct describes a contiguous block of physical memory, defined by its
/// starting address (`start_address`) and size in bytes (`size`). It is used
/// by the memory management system to track and allocate physical memory frames.
///
/// ## Usage
///
/// ```rust
/// use lithium::memory::Frame;
///
/// // Create a new Frame with a starting address and size
/// let frame = Frame {
///     start_address: PhysAddr(0x1000),
///     size: 4096,
/// };
/// ```
#[derive(Debug, Copy, Clone)]
pub(crate) struct Frame {
    start_address: PhysAddr,
    size: usize,
}

impl Frame {
    /// Gets the starting address of the physical frame.
    pub fn start_address(&self) -> PhysAddr {
        self.start_address
    }

    /// Gets the ending address of the physical frame.
    pub fn end_address(&self) -> PhysAddr {
        self.start_address + self.size
    }

    /// Checks if the current frame intersects with another frame.
    pub fn intersects(&self, other: &Frame) -> bool {
        let self_end_address = (self.start_address.as_u64() as usize) + self.size;
        let other_end_address = (other.start_address.as_u64() as usize) + other.size;

        // Check for intersection by comparing start and end addresses
        !(self_end_address <= (other.start_address.as_u64() as usize)
            || (self.start_address.as_u64() as usize) >= other_end_address)
    }
}

impl<S: PageSize> From<PhysFrame<S>> for Frame {
    fn from(value: PhysFrame<S>) -> Self {
        Self {
            start_address: value.start_address(),
            size: value.size() as usize,
        }
    }
}

impl<S: PageSize> TryFrom<Frame> for PhysFrame<S> {
    type Error = AddressNotAligned;

    fn try_from(value: Frame) -> Result<Self, Self::Error> {
        Self::from_start_address(value.start_address())
    }
}

/// Allocator for managing physical memory in the kernel.
///
/// It provides an interface to allocate and deallocate contiguous blocks of physical memory
/// for use by the kernel.
///
/// The physical allocator uses a [bitmap allocation scheme](https://en.wikipedia.org/wiki/Free-space_bitmap)
/// to allocate physical frames.
///
/// ## Usage
///
/// ```rust
/// use lithium::memory::PhysicalAllocator;
///
/// // Initialize the physical memory allocator
/// let mut allocator = PhysicalAllocator::new();
///
/// // Tell allocator there is free memory.
/// allocator.reserve(start: 0x100000, size: 0x80000, block_size: 4096);
///
/// // Allocate a physical memory region.
/// let region = allocator.allocate(4096).expect("Failed to allocate frame");
///
/// // Deallocate the region when no longer needed.
/// allocator.deallocate(region);
/// ```
///
#[derive(Debug)]
pub(crate) struct PhysicalAllocator {
    regions: [Option<PhysicalMemoryBitmap>; MAX_PHYS_REGIONS],
}

impl PhysicalAllocator {
    /// Creates a new physical frame allocator with default regions.
    #[inline]
    pub const fn new() -> Self {
        const ARRAY_REPEAT_VALUE: Option<PhysicalMemoryBitmap> = None;

        Self {
            regions: [ARRAY_REPEAT_VALUE; MAX_PHYS_REGIONS],
        }
    }

    /// Informs memory allocator about a new memory region from `start` to `start + size`.
    pub fn reserve(&mut self, start: PhysAddr, size: usize, block_size: usize) {
        // Find first unused region and mark that out.
        if let Some(region) = self.regions.iter_mut().find(|i| i.is_none()) {
            *region = Some(PhysicalMemoryBitmap::new(start, size, block_size));
        } else {
            panic!("Too many memory regions have been reserved. Can only reserve up to {MAX_PHYS_REGIONS}.");
        }
    }

    /// Allocates a contiguous block of physical memory with the specified size.
    pub fn allocate(&mut self, size: usize) -> Option<Frame> {
        // Find first memory region that has memory available of that sized.
        for region in self.regions.iter_mut().flatten() {
            if region.bytes_remaining() >= size {
                let blocks = region.bytes_to_blocks(size);

                match region.allocate(blocks) {
                    Some(addr) => return Some(addr),
                    None => continue,
                }
            }
        }

        None
    }

    /// Gets the total number of bytes remaining in memory allocator.
    pub fn bytes_remaining(&self) -> usize {
        self.regions
            .iter()
            .filter_map(|x| x.as_ref())
            .map(|x| x.bytes_remaining())
            .sum()
    }

    /// Deallocates a previously allocated physical memory region.
    pub fn deallocate(&mut self, frame: Frame) {
        // Placeholder implementation
        for region in self.regions.iter_mut().flatten() {
            if region.try_deallocate(frame) {
                return;
            }
        }

        panic!("Could not deallocate given frame.")
    }
}

unsafe impl<S: PageSize> FrameAllocator<S> for PhysicalAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        self.allocate(S::SIZE as usize)
            .map(|x| x.try_into().ok())
            .flatten()
    }
}

impl<S: PageSize> FrameDeallocator<S> for PhysicalAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<S>) {
        self.deallocate(frame.into());
    }
}

fn to_virt(addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(addr.as_u64())
}

#[derive(Debug)]
#[repr(C, align(8))]
struct PhysicalMemoryBitmap {
    start_addr: PhysAddr,
    size: usize,
    block_size: usize,
    blocks_remaining: usize,
    reserved: usize,
    bitmap: &'static mut [u8],
}

impl PhysicalMemoryBitmap {
    fn new(start_addr: PhysAddr, size: usize, block_size: usize) -> Self {
        debug_assert!(block_size.is_power_of_two());

        let start_aligned = start_addr.align_up(block_size as u64);
        let end_aligned = (start_addr + size).align_down(block_size as u64);

        let aligned_size = (end_aligned - start_aligned) as usize;
        let bitmap_size = aligned_size / block_size / 8;

        let bitmap = unsafe {
            core::slice::from_raw_parts_mut(to_virt(start_aligned).as_mut_ptr(), bitmap_size)
        };

        // Mark all regions as unused except for the regions currently occupied by the bitmap
        bitmap.fill(0);

        let mut bitmap_reserved_blocks = bitmap_size.next_multiple_of(block_size) / block_size;
        let mut index = 0;

        let reserved = bitmap_reserved_blocks;

        while bitmap_reserved_blocks > 0 {
            for bit in 0..=7 {
                if bitmap_reserved_blocks > 0 {
                    bitmap[index] |= 1 << bit;
                    bitmap_reserved_blocks -= 1;
                } else {
                    break;
                }
            }

            index += 1;
        }

        Self {
            start_addr: start_aligned,
            size: aligned_size,
            block_size,
            blocks_remaining: (aligned_size / block_size) - reserved,
            reserved,
            bitmap,
        }
    }

    const fn bytes_remaining(&self) -> usize {
        self.blocks_remaining * self.block_size
    }

    const fn bytes_to_blocks(&self, size: usize) -> usize {
        size.next_multiple_of(self.block_size) / self.block_size
    }

    const fn bitmap_end(&self) -> usize {
        self.bitmap.len() * 8
    }

    const fn bitmap_start(&self) -> usize {
        self.reserved
    }

    const fn total_blocks(&self) -> usize {
        self.size / self.block_size
    }

    fn contains_frame(&self, frame: Frame) -> bool {
        let start_block = ((frame.start_address - self.start_addr) as usize) / self.block_size;
        let blocks = frame.size.next_multiple_of(self.block_size) / self.block_size;
        let end_block = start_block + blocks;

        (start_block < self.total_blocks()) && (end_block <= self.total_blocks())
    }

    fn allocate(&mut self, blocks: usize) -> Option<Frame> {
        let mut consecutive_blocks = 0;
        let mut start_block = 0;

        for i in self.bitmap_start()..self.bitmap_end() {
            let bit = i & 7;
            let entry = i >> 3;

            if (self.bitmap[entry] & (1 << bit)) == 0 {
                if consecutive_blocks == 0 {
                    start_block = i;
                }

                consecutive_blocks += 1;

                if consecutive_blocks == blocks {
                    // Mark all consecutive blocks as allocated.
                    for j in start_block..start_block + blocks {
                        let bit = j & 7;
                        let entry = j >> 3;

                        self.bitmap[entry] |= 1 << bit;
                    }

                    self.blocks_remaining -= blocks;

                    return Some(Frame {
                        start_address: self.start_addr + (start_block * self.block_size),
                        size: blocks * self.block_size,
                    });
                }
            } else {
                consecutive_blocks = 0;
            }
        }

        None
    }

    fn try_deallocate(&mut self, frame: Frame) -> bool {
        let addr = frame.start_address;
        let blocks: usize = frame.size.next_multiple_of(self.block_size) / self.block_size;
        let relative_addr = (addr - self.start_addr) as usize;

        let start_block = relative_addr / self.block_size;
        let end_block = start_block + blocks;

        if start_block < self.reserved {
            return false;
        }

        if !self.contains_frame(frame) {
            return false;
        }

        for block in start_block..end_block {
            let entry = block >> 3;
            let bit = block & 7;

            assert!(
                (self.bitmap[entry] & (1 << bit)) != 0,
                "Deallocating block that was not held before."
            );

            self.bitmap[entry] &= !(1 << bit);
        }

        self.blocks_remaining += blocks;

        true
    }
}

/// Initializes the memory subsystem of the kernel.
///
/// This function performs the initialization of both the physical memory and virtual
/// memory components of the kernel. It sets up essential data structures, allocates
/// necessary resources, and prepares the system for memory management operations.
pub fn init(mbi_ptr: *const MultibootInformation) {
    let mbi = unsafe { mbi_ptr.as_ref().unwrap() };

    log!("memory::init(): found multiboot structure at {:016p}", mbi);

    // Print out bootloader name.
    if mbi.flags.contains(InfoFlags::BOOT_LOADER_NAME) {
        let name = unsafe { core::ffi::CStr::from_ptr(mbi.boot_loader_name as *const i8) };
        log!("memory::init(): bootloader name {name:?}");
    }

    // Print out total amount of memory available.
    if mbi.flags.contains(InfoFlags::MEMORY) {
        let total_memory = (mbi.mem_lower + mbi.mem_upper) << 10;
        log!("memory::init(): {total_memory} bytes available");
    }

    let kernel_start: usize = unsafe {
        let result;
        core::arch::asm!("lea {}, __kernel_start", out(reg) result);
        result
    };

    let kernel_end: usize = unsafe {
        let result;
        core::arch::asm!("lea {}, __kernel_end", out(reg) result);
        result
    };

    log!(
        "memory::init(): kernel is between {:#016x} and {:#016x}",
        kernel_start,
        kernel_end
    );

    // Panic if memory map is not available.
    if !mbi.flags.contains(InfoFlags::MEM_MAP) {
        panic!("multiboot structure did not have memory map");
    }

    log!("memory::init(): physical memory layout:");

    for (i, area) in mbi.memory_areas().enumerate() {
        let size = (area.size() as f64) / (1 << 20) as f64;
        log!(
            "{:016} | Base: {:#016x} | End: {:#016x} | {:>10.2} MiB {}",
            i,
            area.start_address(),
            area.end_address(),
            size,
            area.area_type()
        );
    }

    log!("memory::init(): initializing physical bitmap allocator...");

    // Keep track of kernel frame so we don't give it to the allocator.
    let kernel_frame = Frame {
        start_address: PhysAddr::new(kernel_start as u64),
        size: (kernel_end - kernel_start).next_multiple_of(4096),
    };

    for area in mbi
        .memory_areas()
        .filter(|x| matches!(x.area_type(), MemoryAreaType::Available))
    {
        let mut start = area.start_address();
        let mut size = area.size();
        let frame = Frame {
            start_address: start,
            size,
        };

        if frame.intersects(&kernel_frame) {
            start = kernel_frame.end_address();
            size = (frame.end_address() - start) as usize;
        }

        // NOTE(kosinw): Rust does not like address zero, so we skip page 0!
        if frame.start_address() == PhysAddr::zero() {
            start = (frame.start_address() + 1u64).align_up(4096u64);
            size = (frame.end_address() - start) as usize;
        }

        // TODO(kosinw): Maybe change this number dynamically to something else?
        unsafe { FRAME_ALLOCATOR.lock().reserve(start, size, 4096) };
    }

    let sz = unsafe { FRAME_ALLOCATOR.lock().bytes_remaining() };

    log!("memory::init(): physical bitmap allocator successfully initialized!");
    log!("memory::init(): {sz} total bytes available");

    // Initialize paging and switch away from boot page table to kernel managed.
    let bootpgtbl: usize = unsafe {
        let result;
        core::arch::asm!("lea {}, bootpgtbl", out(reg) result);
        result
    };

    log!("memory::init(): currently using boot page table at {bootpgtbl:#016x}");

    let mut kpgtbl = unsafe { KERNEL_PAGETABLE.lock() };
    kpgtbl.zero();

    log!(
        "memory::init(): now using kernel page table at {:016p}",
        &*kpgtbl as *const PageTable
    );
}
