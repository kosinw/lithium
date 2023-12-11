use core::fmt;
use core::marker::PhantomData;
use core::mem::size_of;

use bitflags::bitflags;

use x86_64::PhysAddr;

bitflags! {
    /// Flags for multiboot info structure.
    #[derive(Debug, Clone, Copy)]
    pub struct InfoFlags: u32 {
        const MEMORY            = 1 << 0;
        const BOOTDEV           = 1 << 1;
        const CMDLINE           = 1 << 2;
        const MODS              = 1 << 3;
        const AOUT_SYMS         = 1 << 4;
        const ELF_SHDR          = 1 << 5;
        const MEM_MAP           = 1 << 6;
        const DRIVE_INFO        = 1 << 7;
        const CONFIG_TABLE      = 1 << 8;
        const BOOT_LOADER_NAME  = 1 << 9;
        const APM_TABLE         = 1 << 10;
        const VIDEO_INFO        = 1 << 11;
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum MemoryAreaType {
    Invalid = 0,
    Available = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    ReservedHibernate = 4,
    Defective = 5,
}

impl fmt::Display for MemoryAreaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Invalid => "INVALID",
            Self::Available => "AVAILABLE",
            Self::Reserved => "RESERVED",
            Self::AcpiReclaimable => "ACPI_RECLAIMABLE",
            Self::ReservedHibernate => "RESERVED_HIBERNATE",
            Self::Defective => "DEFECTIVE",
        };

        f.write_str(s)
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct MemoryArea {
    size: u32,
    addr: u64,
    len: u64,
    area_type: MemoryAreaType,
}

impl MemoryArea {
    /// The start address of the memory region.
    pub fn start_address(&self) -> PhysAddr {
        PhysAddr::new(self.addr).align_up(4096u64)
    }

    /// The end address of the memory region.
    pub fn end_address(&self) -> PhysAddr {
        PhysAddr::new(self.addr + self.len).align_down(4096u64)
    }

    /// The size, in bytes, of the memory region.
    pub fn size(&self) -> usize {
        self.len as usize
    }

    /// The type of the memory region.
    pub fn area_type(&self) -> MemoryAreaType {
        self.area_type
    }
}

#[repr(C, align(4))]
#[derive(Debug, Clone)]
pub struct MultibootInformation {
    pub flags: InfoFlags,
    pub mem_lower: u32,
    pub mem_upper: u32,
    pub boot_device: u32,
    pub cmdline: u32,
    pub mods_count: u32,
    pub mods_addr: u32,
    _unused0: [u32; 4],
    pub mmap_length: u32,
    pub mmap_addr: u32,
    pub drives_length: u32,
    pub drives_addr: u32,
    _unused1: u32,
    pub boot_loader_name: u32,
    _unused2: [u16; 10],
}

impl MultibootInformation {
    /// Return iterator over all memory areas.
    /// Must check flags to see if MEM_MAP is present otherwise function will panic.
    pub fn memory_areas(&self) -> MemoryAreaIter {
        MemoryAreaIter {
            current_area: self.mmap_addr,
            last_area: self.mmap_length + self.mmap_addr,
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryAreaIter {
    current_area: u32,
    last_area: u32,
    phantom: PhantomData<&'static MemoryArea>,
}

impl Iterator for MemoryAreaIter {
    type Item = &'static MemoryArea;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_area >= self.last_area {
            None
        } else {
            let area = unsafe { &*(self.current_area as *const MemoryArea) };
            self.current_area += (area.size as usize + size_of::<u32>()) as u32;
            if matches!(area.area_type, MemoryAreaType::Invalid) {
                None
            } else {
                Some(area)
            }
        }
    }
}
