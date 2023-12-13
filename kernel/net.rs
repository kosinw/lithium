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
/// The offset of the`notify_off_multiplier` field within `virtio_pci_notify_cap`.
const VIRTIO_PCI_CAP_NOTIFY_OFF_MULTIPLIER_OFFSET: u8 = 16;

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
struct VirtioTransportConfig {
    // PCI information.
    pci_cfg: pci::DeviceConfig,
    // Common configuration structure.
    common_cfg: NonNull<VirtioPciCommonCfg>,
    // Start of queue notification region.
    notify_region: NonNull<[u16]>,
    notify_off_mulitplier: u32,
    // The interrupt status register.
    isr_status: NonNull<u8>,
    // Device-specific configuration.
    config_space: Option<NonNull<[u32]>>,
}

impl VirtioTransportConfig {
    fn from_device_config(pci_device_cfg: &mut pci::DeviceConfig) -> VirtioTransportConfig {
        use bit_field::BitField;

        // Enable PCI bus mastering to allow virtio-net to do DMA.
        pci_device_cfg.enable_bus_mastering();

        // Find the PCI capabilities we need.
        let mut common_cfg = None;
        let mut notify_cfg = None;
        let mut notify_off_multiplier = 0;
        let mut isr_cfg = None;
        let mut device_cfg = None;

        // Find all of the virtio vendor specific capabilities.
        for capability in pci_device_cfg
            .capabilities()
            .expect("could not find capabilities list for virtio-net driver")
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
                bar: pci_device_cfg.config_read_word(capability.offset + VIRTIO_PCI_CAP_BAR_OFFSET)
                    as u8,
                offset: pci_device_cfg
                    .config_read_word(capability.offset + VIRTIO_PCI_CAP_OFFSET_OFFSET),
                length: pci_device_cfg
                    .config_read_word(capability.offset + VIRTIO_PCI_CAP_LENGTH_OFFSET),
            };

            match cfg_type {
                VIRTIO_PCI_CAP_COMMON_CFG => {
                    common_cfg = if common_cfg.is_some() {
                        common_cfg
                    } else {
                        Some(cap_info)
                    };
                }
                VIRTIO_PCI_CAP_NOTIFY_CFG => {
                    // 4.1.4.4 Notification structure layout
                    // The notification location is found using the VIRTIO_PCI_CAP_NOTIFY_CFG capability.
                    // This capability is immediately followed by an additional field, like so:
                    //
                    // struct virtio_pci_notify_cap {
                    //         struct virtio_pci_cap cap;
                    //         le32 notify_off_multiplier; /* Multiplier for queue_notify_off. */
                    // };
                    //

                    notify_cfg = if notify_cfg.is_some() {
                        notify_cfg
                    } else {
                        Some(cap_info)
                    };
                    notify_off_multiplier = pci_device_cfg.config_read_word(
                        capability.offset + VIRTIO_PCI_CAP_NOTIFY_OFF_MULTIPLIER_OFFSET,
                    );
                }
                VIRTIO_PCI_CAP_ISR_CFG => {
                    isr_cfg = if isr_cfg.is_some() {
                        isr_cfg
                    } else {
                        Some(cap_info)
                    };
                }
                VIRTIO_PCI_CAP_DEVICE_CFG => {
                    device_cfg = if device_cfg.is_some() {
                        device_cfg
                    } else {
                        Some(cap_info)
                    };
                }
                _ => {}
            }
        }

        todo!()
    }
}

pub fn init() {
    // First find configuration for virtio net device.
    let mut device_cfg = pci::find_device(VIRTIO_VENDOR_ID, VIRTIO_NET_DEVICE_ID)
        .expect("could not find virtio-net device on PCI bus");

    log!("net::init(): found virtio-net device");

    // Build the transport layer using PCI bus info.
    let transport_layer = VirtioTransportConfig::from_device_config(&mut device_cfg);
}
