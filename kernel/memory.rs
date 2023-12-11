use x86_64::{PhysAddr, VirtAddr};

/// Maximum number of physical memory regions that can be used by physical allocator.
const MAX_PHYS_REGIONS: usize = 16;

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
pub(crate) struct Frame {
    start_address: PhysAddr,
    size: usize,
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
    pub fn deallocate(&mut self, region: PhysAddr, size: u64) {
        // Placeholder implementation
        todo!()
    }
}

fn to_virt(addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(addr.as_u64())
}

fn to_phys(addr: VirtAddr) -> PhysAddr {
    PhysAddr::new(addr.as_u64())
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
            for bit in (0..=7) {
                if bitmap_reserved_blocks > 0 {
                    bitmap[index] |= 1 << bit;
                    bitmap_reserved_blocks -= 1;
                }
            }

            index += 1;
        }

        Self {
            start_addr: start_aligned,
            size: aligned_size,
            block_size,
            blocks_remaining: aligned_size - (reserved * block_size),
            reserved,
            bitmap,
        }
    }

    fn bytes_remaining(&self) -> usize {
        self.blocks_remaining * self.block_size
    }

    fn bytes_to_blocks(&self, size: usize) -> usize {
        size.next_multiple_of(self.block_size) / self.block_size
    }

    fn bitmap_end(&self) -> usize {
        self.bitmap.len() * 8
    }

    fn bitmap_start(&self) -> usize {
        self.reserved
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

                    return Some(self.start_addr + (start_block * self.block_size));
                }
            } else {
                consecutive_blocks = 0;
            }
        }

        None
    }

    fn deallocate(&mut self, addr: PhysAddr, blocks: usize) {
        let relative_addr = (addr - self.start_addr) as usize;

        let start_block = relative_addr / self.block_size;
        let end_block = start_block + blocks;
        let block_range = start_block..end_block;

        debug_assert!(
            start_block >= self.reserved,
            "Cannot deallocate blocks used by bitmap."
        );

        debug_assert!(
            start_block < (self.start_addr.as_u64() + self.size) as usize,
            "Deallocating invalid memory region."
        );

        debug_assert!(
            end_block <= (self.start_addr.as_u64() + self.size) as usize,
            "Deallocating invalid memory region."
        );

        for block in start_block..end_block {
            let entry = block >> 3;
            let bit = block & 7;

            assert!(
                self.bitmap[entry] & (1 << bit),
                "Deallocating block that was not held before."
            );

            self.bitmap[entry] &= !(1 << bit);
        }

        self.blocks_remaining += blocks;
    }
}
