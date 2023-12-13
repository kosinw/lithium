use core::ptr::NonNull;

use crate::log;
use crate::pci;

pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_NET_DEVICE_ID: u16 = 0x1000;

/// The offset of the bar field within `virtio_pci_cap`.
const VIRTIO_PCI_CAP_BAR_OFFSET: u8 = 4;
/// The offset of the offset field with `virtio_pci_cap`.
const VIRTIO_PCI_CAP_OFFSET_OFFSET: u8 = 8;
/// The offset of the `length` field within `virtio_pci_cap`.
const VIRTIO_PCI_CAP_LENGTH_OFFSET: u8 = 12;

/// Common configuration.
const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
/// Notifications.
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
/// ISR Status.
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
/// Device specific configuration.
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

/// `virtio_pci_cap`, see section 4.1.4 Virtio Structure PCI Capabilities
#[derive(Clone, Debug, Eq, PartialEq)]
struct VirtioPciCapability {
    bar: u8,
    offset: u32,
    length: u32,
}

/// `virtio_pci_common_cfg`, see 4.1.4.3 "Common configuration structure layout".
#[repr(C)]
struct VirtioPciCommonCfg {
    device_feature_select: u32,
    device_feature: u32,
    driver_feature_select: u32,
    driver_feature: u32,
    msix_config: u16,
    num_queues: u16,
    device_status: u8,
    config_generation: u8,
    queue_select: u16,
    queue_size: u16,
    queue_msix_vector: u16,
    queue_enable: u16,
    queue_notify_off: u16,
    queue_desc: u64,
    queue_driver: u64,
    queue_device: u64,
}

#[derive(Debug)]
struct VirtioTransportInfo {
    pci_cfg: pci::DeviceConfig,
    common_cfg: NonNull<VirtioPciCommonCfg>,
    notify_region: NonNull<[u16]>,
    notify_off_mulitplier: u32,
    isr_status: NonNull<u8>,
    config_space: Option<NonNull<[u32]>>
}

pub fn init() {
    use bit_field::BitField;

    // First find configuration for virtio net device.
    let mut device_cfg = pci::find_device(VIRTIO_VENDOR_ID, VIRTIO_NET_DEVICE_ID)
        .expect("could not find virtio-net device on PCI bus");

    log!(
        "net::init(): found virtio-net device config {:?}",
        device_cfg
    );

    // Enable PCI bus mastering to allow virtio-net to do DMA.
    device_cfg.enable_bus_mastering();

    // Find all of the virtio vendor specific capabilities.
    for capability in device_cfg
        .capabilities()
        .expect("Could not find capabilities list for virtio-net driver")
    {
        if capability.id != pci::PCI_CAP_ID_VNDR {
            continue;
        }

        let cap_len = capability.private_header.get_bits(0..8) as u8;
        let cfg_type = capability.private_header.get_bits(8..16) as u8;

        if cap_len < 16 {
            continue;
        }

        let cap_info = VirtioPciCapability {
            bar: device_cfg.config_read_word(capability.offset + VIRTIO_PCI_CAP_BAR_OFFSET) as u8,
            offset: device_cfg.config_read_word(capability.offset + VIRTIO_PCI_CAP_OFFSET_OFFSET)
                as u8,
            length: device_cfg.config_read_word(capability.offset + VIRTIO_PCI_CAP_LENGTH_OFFSET),
        };
    }
}
