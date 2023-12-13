use crate::log;
use alloc::vec::Vec;
use bitflags::bitflags;
use spin::Mutex;
use x86_64::instructions::port::Port;

// I/O port for PCI config address.
pub const PCI_CONFIG_ADDRESS_PORT: u16 = 0xCF8;

// I/O port for PCI config data.
pub const PCI_CONFIG_DATA_PORT: u16 = 0xCFC;

// List of all valid PCI devices.
static mut PCI_DEVICES: Mutex<Vec<DeviceConfig>> = Mutex::new(Vec::new());

/// The offset in bytes to BAR0 within PCI configuration space.
const BAR0_OFFSET: u8 = 0x10;

/// ID for vendor-specific PCI capabilities.
pub const PCI_CAP_ID_VNDR: u8 = 0x09;

bitflags! {
    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub struct Status: u16 {
        const INTERRUPT_STATUS = 1 << 3;
        const CAPABILITIES_LIST = 1 << 4;
        const MHZ_66_CAPABLE = 1 << 5;
        const FAST_BACK_TO_BACK_CAPABLE = 1 << 7;
        const MASTER_DATA_PARITY_ERROR = 1 << 8;
        const SIGNALED_TARGET_ABORT = 1 << 11;
        const RECEIVED_TARGET_ABORT = 1 << 12;
        const RECEIVED_MASTER_ABORT = 1 << 13;
        const SIGNALED_SYSTEM_ERROR = 1 << 14;
        const DETECTED_PARITY_ERROR = 1 << 15;
    }

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub struct Command: u16 {
        const IO_SPACE = 1 << 0;
        const MEMORY_SPACE = 1 << 1;
        const BUS_MASTER = 1 << 2;
        const SPECIAL_CYCLES = 1 << 3;
        const MEMORY_WRITE_AND_INVALIDATE_ENABLE = 1 << 4;
        const VGA_PALETTE_SNOOP = 1 << 5;
        const PARITY_ERROR_RESPONSE = 1 << 6;
        const SERR_ENABLE = 1 << 8;
        const FAST_BACK_TO_BACK_ENABLE = 1 << 9;
        const INTERRUPT_DISABLE = 1 << 10;
    }
}

// For more information: https://wiki.osdev.org/Pci#PCI_Device_Structure
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub device_id: u16,
    pub vendor_id: u16,
    pub command: Command,
    pub status: Status,
    pub revision: u8,
    pub prog_if: u8,
    pub subclass: u8,
    pub class: u8,
    pub header_type: u8,
    pub base_addresses: [u32; 6],
    pub interrupt_pin: u8,
    pub interrupt_line: u8,
}

impl DeviceConfig {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        use bit_field::BitField;

        // TODO(kosinw): Assume for now all devices are header type 0x0
        let mut register = ConfigRegister::new(bus, device, function, 0x00);
        let data = register.read();

        let vendor_id = data.get_bits(0..16) as u16;
        let device_id = data.get_bits(16..32) as u16;

        let mut register = ConfigRegister::new(bus, device, function, 0x04);
        let data = register.read();

        let command = Command::from_bits_truncate(data.get_bits(0..16) as u16);
        let status = Status::from_bits_truncate(data.get_bits(16..32) as u16);

        let mut register = ConfigRegister::new(bus, device, function, 0x08);
        let data = register.read();

        let revision = data.get_bits(0..8) as u8;
        let prog_if = data.get_bits(8..16) as u8;
        let subclass = data.get_bits(16..24) as u8;
        let class = data.get_bits(24..32) as u8;

        let mut register = ConfigRegister::new(bus, device, function, 0x0C);
        let data = register.read();

        let header_type = data.get_bits(16..24) as u8;

        let mut register = ConfigRegister::new(bus, device, function, 0x3C);
        let data = register.read();

        let interrupt_line = data.get_bits(0..8) as u8;
        let interrupt_pin = data.get_bits(8..16) as u8;

        let mut base_addresses = [0u32; 6];

        for (i, ba) in base_addresses.iter_mut().enumerate() {
            let offset = (BAR0_OFFSET as usize + i * 4) as u8;
            let mut register = ConfigRegister::new(bus, device, function, offset);
            *ba = register.read();
        }

        Self {
            bus,
            device,
            function,
            device_id,
            vendor_id,
            command,
            status,
            revision,
            prog_if,
            subclass,
            class,
            header_type,
            base_addresses,
            interrupt_line,
            interrupt_pin,
        }
    }

    pub fn capabilities(&self) -> Option<CapabilityIter> {
        use bit_field::BitField;

        if self.status.contains(Status::CAPABILITIES_LIST) {
            let caps_offset = (self.config_read_word(0x34).get_bits(2..8) << 2) as u8;

            Some(CapabilityIter {
                bus: self.bus,
                device: self.device,
                function: self.function,
                next_capability_offset: Some(caps_offset),
            })
        } else {
            None
        }
    }

    /// Reads word from PCI configuration space.
    pub fn config_read_word(&self, offset: u8) -> u32 {
        ConfigRegister::new(self.bus, self.device, self.function, offset).read()
    }

    /// Writes word to PCI configuration space.
    pub fn config_write_word(&self, offset: u8, word: u32) {
        ConfigRegister::new(self.bus, self.device, self.function, offset).write(word);
    }

    /// Enables PCI bus mastering (first-party DMA) for this device.
    pub fn enable_bus_mastering(&mut self) {
        use bit_field::BitField;

        let mut register = ConfigRegister::new(self.bus, self.device, self.function, 0x04);
        let mut data = register.read();
        data.set_bit(2, true);
        register.write(data);
    }
}

#[derive(Debug)]
pub struct CapabilityIter {
    bus: u8,
    device: u8,
    function: u8,
    next_capability_offset: Option<u8>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CapabilityInfo {
    /// The offset of the capability in the PCI configuration space of the device function.
    pub offset: u8,
    /// The ID of the capability.
    pub id: u8,
    /// The third and fourth bytes of the capability, to save reading them again.
    pub private_header: u16,
}

impl Iterator for CapabilityIter {
    type Item = CapabilityInfo;

    fn next(&mut self) -> Option<Self::Item> {
        use bit_field::BitField;

        let offset = self.next_capability_offset?;

        let mut register = ConfigRegister::new(self.bus, self.device, self.function, offset);
        let capability_header = register.read();

        let id = capability_header.get_bits(0..8) as u8;
        let next_offset = capability_header.get_bits(8..16) as u8;
        let private_header = capability_header.get_bits(16..32) as u16;

        self.next_capability_offset = if next_offset == 0 {
            None
        } else {
            Some(next_offset)
        };

        Some(CapabilityInfo {
            offset,
            id,
            private_header,
        })
    }
}

pub struct ConfigRegister {
    addr_port: Port<u32>,
    data_port: Port<u32>,
    addr: u32,
}

impl ConfigRegister {
    pub fn new(bus: u8, device: u8, function: u8, offset: u8) -> Self {
        Self {
            addr_port: Port::new(PCI_CONFIG_ADDRESS_PORT),
            data_port: Port::new(PCI_CONFIG_DATA_PORT),
            addr: 0x8000_0000
                | ((bus as u32) << 16)
                | ((device as u32) << 11)
                | ((function as u32) << 8)
                | ((offset as u32) & 0xFC),
        }
    }

    pub fn read(&mut self) -> u32 {
        unsafe {
            self.addr_port.write(self.addr);
            self.data_port.read()
        }
    }

    pub fn write(&mut self, v: u32) {
        unsafe {
            self.addr_port.write(self.addr);
            self.data_port.write(v);
        }
    }
}

fn add_device(device: &DeviceConfig) {
    unsafe {
        PCI_DEVICES.lock().push(*device);
    }

    log!(
        "pci::init(): Bus: {:02X} | Device: {:02X} | Function: {:02X} | [{:04X}:{:04X}]",
        device.bus,
        device.device,
        device.function,
        device.vendor_id,
        device.device_id
    );
}

fn check_device(bus: u8, device: u8) {
    let potential_device = DeviceConfig::new(bus, device, 0);

    if potential_device.vendor_id == 0xFFFFu16 {
        return;
    }

    add_device(&potential_device);

    // Is this a multi function device?
    if potential_device.header_type & 0x80 != 0 {
        for function in 1u8..8u8 {
            let potential_device = DeviceConfig::new(bus, device, function);
            if potential_device.vendor_id != 0xFFFFu16 {
                add_device(&potential_device);
            }
        }
    }
}

fn check_bus(bus: u8) {
    for device in 0u8..32u8 {
        check_device(bus, device);
    }
}

/// Finds PCI device configuration given vendor and device ID.
pub fn find_device(vendor_id: u16, device_id: u16) -> Option<DeviceConfig> {
    unsafe { PCI_DEVICES.lock().iter() }
        .find(|&&device| device.vendor_id == vendor_id && device.device_id == device_id)
        .copied()
}

/// Initializes the PCI (Peripheral Component Interconnect) subsystem in the kernel.
///
/// This function initializes the PCI subsystem, scans for PCI devices, and performs necessary
/// setup to enable communication with PCI-connected devices. It sets up data structures and
/// configurations needed for interacting with PCI devices in the system.
pub fn init() {
    log!("pci::init(): enumerating PCI bus...");
    // Enumerate over all busses and find all PCI devices.
    for bus in 0u8..=255u8 {
        check_bus(bus);
    }
    log!("pci::init(): successfully enumerated PCI bus [ \x1b[0;32mOK\x1b[0m ]");
}
