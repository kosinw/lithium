use alloc::vec::Vec;
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::log;

// I/O port for PCI config address.
pub const PCI_CONFIG_ADDRESS_PORT: u16 = 0xCF8;

// I/O port for PCI config data.
pub const PCI_CONFIG_DATA_PORT: u16 = 0xCFC;

// List of all valid PCI devices.
static mut PCI_DEVICES: Mutex<Vec<PciDeviceConfig>> = Mutex::new(Vec::new());

// For more information: https://wiki.osdev.org/Pci#PCI_Device_Structure
#[derive(Debug, Clone, Copy)]
pub struct PciDeviceConfig {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub device_id: u16,
    pub vendor_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision: u8,
    pub prog_if: u8,
    pub subclass: u8,
    pub class: u8,
    pub header_type: u8,
    pub base_addresses: [u32; 6],
    pub interrupt_pin: u8,
    pub interrupt_line: u8,
}

impl PciDeviceConfig {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        use bit_field::BitField;

        // TODO(kosinw): Assume for now all devices are header type 0x0
        let mut register = PciConfigRegister::new(bus, device, function, 0x00);
        let data = register.read();

        let vendor_id = data.get_bits(0..16) as u16;
        let device_id = data.get_bits(16..32) as u16;

        let mut register = PciConfigRegister::new(bus, device, function, 0x04);
        let data = register.read();

        let command = data.get_bits(0..16) as u16;
        let status = data.get_bits(16..32) as u16;

        let mut register = PciConfigRegister::new(bus, device, function, 0x08);
        let data = register.read();

        let revision = data.get_bits(0..8) as u8;
        let prog_if = data.get_bits(8..16) as u8;
        let subclass = data.get_bits(16..24) as u8;
        let class = data.get_bits(24..32) as u8;

        let mut register = PciConfigRegister::new(bus, device, function, 0x0C);
        let data = register.read();

        let header_type = data.get_bits(16..24) as u8;

        let mut register = PciConfigRegister::new(bus, device, function, 0x3C);
        let data = register.read();

        let interrupt_line = data.get_bits(0..8) as u8;
        let interrupt_pin = data.get_bits(8..16) as u8;

        let mut base_addresses = [0u32; 6];

        for (i, ba) in base_addresses.iter_mut().enumerate() {
            let offset = (0x10 + i * 4) as u8;
            let mut register = PciConfigRegister::new(bus, device, function, offset);
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
}

struct PciConfigRegister {
    addr_port: Port<u32>,
    data_port: Port<u32>,
    addr: u32,
}

impl PciConfigRegister {
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

fn add_device(device: &PciDeviceConfig) {
    unsafe {
        PCI_DEVICES.lock().push(device.clone());
    }

    log!(
        "pci::init(): found PCI device: Bus: {:02X} | Device: {:02X} | Function: {:02X} | [{:04X}:{:04X}]",
        device.bus,
        device.device,
        device.function,
        device.vendor_id,
        device.device_id
    );
}

fn check_device(bus: u8, device: u8) {
    let potential_device = PciDeviceConfig::new(bus, device, 0);

    if potential_device.vendor_id == 0xFFFFu16 {
        return;
    }

    add_device(&potential_device);

    // Is this a multi function device?
    if potential_device.header_type & 0x80 != 0 {
        for function in 1u8..8u8 {
            let potential_device = PciDeviceConfig::new(bus, device, function);
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

/// Initializes the PCI (Peripheral Component Interconnect) subsystem in the kernel.
///
/// This function initializes the PCI subsystem, scans for PCI devices, and performs necessary
/// setup to enable communication with PCI-connected devices. It sets up data structures and
/// configurations needed for interacting with PCI devices in the system.
pub fn init() {
    for bus in 0u8..=255u8 {
        check_bus(bus);
    }
}
