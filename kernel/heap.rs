use alloc::alloc::Layout;
use core::marker::PhantomData;
use core::{fmt, ptr::NonNull};
use x86_64::{align_down, VirtAddr};

/// A heap allocator using the power-of-two buddy system.
///
/// The `BuddyAllocator` struct divides a given memory region into blocks of powers of two sizes,
/// allowing for efficient allocation and deallocation of memory. It employs the buddy system,
/// where adjacent free blocks are combined (buddied) to form larger blocks when possible.
pub(crate) struct BuddyAllocator<const ORDER: usize> {
    free_lists: [LinkedList; ORDER],
    allocated: usize,
    requested: usize,
    total: usize,
}

pub(crate) fn prev_power_of_two(num: u64) -> u64 {
    1 << (u64::BITS as usize - num.leading_zeros() as usize - 1) as u64
}

impl<const ORDER: usize> BuddyAllocator<ORDER> {
    /// Create a new empty heap.
    pub const fn new() -> Self {
        Self {
            free_lists: [LinkedList::new(); ORDER],
            allocated: 0,
            requested: 0,
            total: 0,
        }
    }

    /// Tell the heap about a free memory region.
    /// Note that everything is minimum aligned to u64.
    pub unsafe fn reserve(&mut self, mut start: VirtAddr, mut size: u64) {
        // align to the u64
        start = start.align_up(core::mem::size_of::<u64>() as u64);
        let end = start + align_down(size, core::mem::size_of::<u64>() as u64);

        let mut total = 0;
        let mut current_start = start;

        while (current_start + core::mem::size_of::<u64>()) <= end {
            let current_size = current_start.as_u64() & (!current_start.as_u64() + 1);
            let current_size = core::cmp::min(current_size, prev_power_of_two(end - current_start));
            total += current_size;

            self.free_lists[current_size.trailing_zeros() as usize]
                .push(current_start.as_mut_ptr());
            current_start += current_size;
        }

        self.total += total as usize;
    }

    /// Allocate a range of memory from the heap.
    pub fn allocate(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        let actual_size = core::cmp::max(
            layout.size().next_power_of_two(),
            core::cmp::max(layout.align(), core::mem::size_of::<u64>()),
        );

        let order = actual_size.trailing_zeros() as usize;

        for i in order..self.free_lists.len() {
            // Find the smallest block that can work with
            if !self.free_lists[i].is_empty() {
                // Splitting chunks so that we an have some in free list i
                for j in (order + 1..i + 1).rev() {
                    if let Some(block) = self.free_lists[j].pop() {
                        // Moving both block and its buddy into the list below
                        unsafe {
                            self.free_lists[j - 1]
                                .push((block as usize + (1 << (j - 1))) as *mut usize);
                            self.free_lists[j - 1].push(block);
                        }
                    } else {
                        return Err(());
                    }
                }
            }

            let result = NonNull::new(
                self.free_lists[order]
                    .pop()
                    .expect("current block should have free space now") as *mut u8,
            );
            if let Some(result) = result {
                self.requested += layout.size();
                self.allocated += actual_size;
                return Ok(result);
            } else {
                return Err(());
            }
        }

        Err(())
    }

    /// Returns the total number of bytes in this heap.
    pub fn total_bytes(&self) -> usize {
        self.total
    }

    /// Returns the total number of bytes requested by callees.
    pub fn total_request(&self) -> usize {
        self.requested
    }

    /// Returns the total number of bytes actually allocated.
    pub fn total_allocated(&self) -> usize {
        self.allocated
    }
}

#[derive(Copy, Clone)]
struct LinkedList {
    head: *mut usize,
}

unsafe impl Send for LinkedList {}

impl LinkedList {
    pub const fn new() -> Self {
        Self {
            head: core::ptr::null_mut(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    pub unsafe fn push(&mut self, item: *mut usize) {
        *item = self.head as usize;
        self.head = item;
    }

    pub fn pop(&mut self) -> Option<*mut usize> {
        match self.is_empty() {
            true => None,
            false => {
                // Advance head pointer
                let item = self.head;
                self.head = unsafe { *item as *mut usize };
                Some(item)
            }
        }
    }

    pub fn iter(&self) -> LinkedListIter {
        LinkedListIter {
            curr: self.head,
            list: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> LinkedListIterMut {
        LinkedListIterMut {
            prev: &mut self.head as *mut *mut usize as *mut usize,
            curr: self.head,
            list: PhantomData,
        }
    }
}

impl fmt::Debug for LinkedList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

pub struct LinkedListIter<'a> {
    curr: *mut usize,
    list: PhantomData<&'a LinkedList>,
}

impl<'a> Iterator for LinkedListIter<'a> {
    type Item = *mut usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            None
        } else {
            let item = self.curr;
            let next = unsafe { *item as *mut usize };
            self.curr = next;
            Some(item)
        }
    }
}

pub struct LinkedListIterMutItem {
    prev: *mut usize,
    curr: *mut usize,
}

impl LinkedListIterMutItem {
    pub fn pop(self) -> *mut usize {
        unsafe {
            *(self.prev) = *(self.curr);
        }
        self.curr
    }

    pub fn value(&self) -> *mut usize {
        self.curr
    }
}

pub struct LinkedListIterMut<'a> {
    list: PhantomData<&'a mut LinkedList>,
    prev: *mut usize,
    curr: *mut usize,
}

impl<'a> Iterator for LinkedListIterMut<'a> {
    type Item = LinkedListIterMutItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            None
        } else {
            let res = LinkedListIterMutItem {
                prev: self.prev,
                curr: self.curr,
            };
            self.prev = self.curr;
            self.curr = unsafe { *self.curr as *mut usize };
            Some(res)
        }
    }
}
