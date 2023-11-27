use core::fmt;
use core::marker::PhantomData;
use core::mem::size_of;

use bitflags::bitflags;

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
    Available = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    ReservedHibernate = 4,
    Defective = 5,
}

impl fmt::Display for MemoryAreaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
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
    pub size: u32,
    pub addr: u64,
    pub len: u64,
    pub area_type: MemoryAreaType,
}

impl MemoryArea {
    /// The start address of the memory region.
    pub fn start_address(&self) -> u64 {
        self.addr
    }

    /// The end address of the memory region.
    pub fn end_address(&self) -> u64 {
        self.addr + self.len - 1
    }

    /// The size, in bytes, of the memory region.
    pub fn size(&self) -> u64 {
        self.len
    }

    /// The type of the memory region.
    pub fn area_type(&self) -> MemoryAreaType {
        self.area_type
    }
}

#[repr(C, align(4))]
#[derive(Debug, Clone)]
pub struct MultibootInfo {
    pub flags: InfoFlags,
    pub mem_lower: u32,
    pub mem_upper: u32,
    pub boot_device: u32,
    pub cmdline: u32,
    pub mods_count: u32,
    pub mods_addr: u32,
    _unused0: [u32; 4], // todo(kosinw): implement this later
    pub mmap_length: u32,
    pub mmap_addr: u32,
    pub drives_length: u32,
    pub drives_addr: u32,
    _unused1: u32,
    pub boot_loader_name: u32,
    _unused2: [u16; 10],
}

impl MultibootInfo {
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
            let next_area = (area.size as usize + size_of::<u32>()) as u32;
            if next_area <= self.last_area {
                self.current_area = next_area;
                Some(area)
            } else {
                self.current_area = self.last_area;
                None
            }
        }
    }
}
