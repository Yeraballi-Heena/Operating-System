//! Integrate OS — USB Subsystem Manager & Device Registry
//!
//! Orchestrates host controller detection, recursive bus enumeration, address assignment,
//! descriptor queries, and class driver binding for HID and Mass Storage peripherals.

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::pci::PCI_MANAGER;
use crate::sync::IrqSpinlock;
use crate::{klog_info, klog_ok};
use super::bus::UsbBus;
use super::controller::UsbHostController;
use super::device::UsbDevice;
use super::hid::{UsbHidKeyboard, UsbHidMouse};
use super::host::{EhciController, UhciController, VirtualUsbHostController, XhciController};
use super::msc::UsbStorageDevice;
use super::types::*;

/// Central USB Subsystem Manager.
pub struct UsbManager {
    controllers: Vec<Box<dyn UsbHostController>>,
    devices: Vec<UsbDevice>,
    next_address: u8,
    keyboards: Vec<UsbHidKeyboard>,
    mice: Vec<UsbHidMouse>,
    storage_devices: Vec<UsbStorageDevice>,
}

impl UsbManager {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            devices: Vec::new(),
            next_address: 1,
            keyboards: Vec::new(),
            mice: Vec::new(),
            storage_devices: Vec::new(),
        }
    }

    /// Discovers physical host controllers on the PCI bus and registers the virtual host.
    pub fn discover_controllers(&mut self) {
        let pci_usb_devices: Vec<crate::pci::PciDevice> = {
            let mgr = PCI_MANAGER.lock();
            mgr.find_by_class(0x0C, 0x03).into_iter().cloned().collect()
        };

        for pci_dev in pci_usb_devices {
            let addr = pci_dev.addr;
            match pci_dev.prog_if {
                0x00 => {
                    // UHCI
                    let io_base = pci_dev.bars[0].as_ref().and_then(|b| b.io_port()).unwrap_or(0);
                    klog_info!("[usb] Found UHCI USB 1.1 controller at {} (IO 0x{:04X})", addr.format(), io_base);
                    let mut ctrl = Box::new(UhciController::new(addr, io_base));
                    let _ = ctrl.init();
                    self.controllers.push(ctrl);
                }
                0x20 => {
                    // EHCI
                    let mmio = pci_dev.bars[0].as_ref().and_then(|b| b.mmio_address()).unwrap_or(0);
                    klog_info!("[usb] Found EHCI USB 2.0 controller at {} (MMIO 0x{:08X})", addr.format(), mmio);
                    let mut ctrl = Box::new(EhciController::new(addr, mmio));
                    let _ = ctrl.init();
                    self.controllers.push(ctrl);
                }
                0x30 => {
                    // xHCI
                    let mmio = pci_dev.bars[0].as_ref().and_then(|b| b.mmio_address()).unwrap_or(0);
                    klog_info!("[usb] Found xHCI USB 3.0 controller at {} (MMIO 0x{:08X})", addr.format(), mmio);
                    let mut ctrl = Box::new(XhciController::new(addr, mmio));
                    let _ = ctrl.init();
                    self.controllers.push(ctrl);
                }
                _ => {
                    klog_info!("[usb] Discovered generic USB controller at {} (ProgIF 0x{:02X})", addr.format(), pci_dev.prog_if);
                }
            }
        }

        // Always register the Virtual / Root USB Host Controller
        let mut vhost = Box::new(VirtualUsbHostController::new());
        let _ = vhost.init();
        self.controllers.push(vhost);
        klog_info!("[usb] Registered Virtual USB 2.0 Host Controller & Root Hub");
    }

    /// Enumerates all connected ports across all registered host controllers.
    pub fn enumerate_all(&mut self) {
        let ctrl_count = self.controllers.len();
        for ctrl_idx in 0..ctrl_count {
            let port_count = self.controllers[ctrl_idx].port_count();
            for port in 0..port_count {
                if self.controllers[ctrl_idx].is_port_connected(port) {
                    if let Err(e) = self.enumerate_device(ctrl_idx, port) {
                        klog_info!("[usb] Failed to enumerate device on ctrl {}, port {}: {}", ctrl_idx, port, e.as_str());
                    }
                }
            }
        }
    }

    /// Executes the standard 9-step USB peripheral enumeration sequence.
    fn enumerate_device(&mut self, ctrl_idx: usize, port: u8) -> Result<(), UsbError> {
        let ctrl = &mut *self.controllers[ctrl_idx];

        // 1. Reset device port
        ctrl.reset_port(port)?;

        // 2. Read Device Descriptor on default address 0 to learn bMaxPacketSize0
        let partial_desc = UsbBus::get_device_descriptor(ctrl, 0, 18)?;
        let _max_packet0 = partial_desc.b_max_packet_size0;

        // 3. Allocate unique bus address and issue SET_ADDRESS
        let new_addr = self.next_address;
        self.next_address = self.next_address.checked_add(1).unwrap_or(127);
        UsbBus::set_address(ctrl, 0, new_addr)?;

        // 4. Read complete 18-byte Device Descriptor on newly assigned address
        let device_desc = UsbBus::get_device_descriptor(ctrl, new_addr, 18)?;

        // 5. Read complete Configuration Descriptor tree (interfaces, endpoints, HID)
        let parsed_tree = UsbBus::get_configuration(ctrl, new_addr, 0)?;

        // 6. Read string descriptors
        let mfg_str = if device_desc.i_manufacturer != 0 {
            UsbBus::get_string_descriptor(ctrl, new_addr, device_desc.i_manufacturer).ok()
        } else {
            None
        };

        let prod_str = if device_desc.i_product != 0 {
            UsbBus::get_string_descriptor(ctrl, new_addr, device_desc.i_product).ok()
        } else {
            None
        };

        let ser_str = if device_desc.i_serial_number != 0 {
            UsbBus::get_string_descriptor(ctrl, new_addr, device_desc.i_serial_number).ok()
        } else {
            None
        };

        // 7. Activate configuration 1
        UsbBus::set_configuration(ctrl, new_addr, 1)?;

        // 8. Construct UsbDevice record
        let usb_dev = UsbDevice {
            address: new_addr,
            speed: UsbSpeed::HighSpeed,
            controller_id: ctrl_idx,
            port,
            descriptor: device_desc,
            active_config: parsed_tree.config,
            interfaces: parsed_tree.interfaces,
            current_configuration: 1,
            manufacturer: mfg_str,
            product: prod_str,
            serial: ser_str,
        };

        klog_ok!("[usb] Discovered {}", usb_dev.format_description());

        // 9. Bind device class drivers
        if usb_dev.is_hid_keyboard() {
            let kbd = UsbHidKeyboard::new(new_addr);
            self.keyboards.push(kbd);
            klog_ok!("[usb] Bound USB HID Boot Keyboard driver for Addr {}", new_addr);
        } else if usb_dev.is_hid_mouse() {
            let mouse = UsbHidMouse::new(new_addr);
            self.mice.push(mouse);
            klog_ok!("[usb] Bound USB HID Boot Mouse driver for Addr {}", new_addr);
        } else if usb_dev.is_mass_storage() {
            let ep_in = usb_dev
                .find_endpoint(UsbTransferType::Bulk, RequestDirection::DeviceToHost)
                .map_or(0x81, |e| e.b_endpoint_address);
            let ep_out = usb_dev
                .find_endpoint(UsbTransferType::Bulk, RequestDirection::HostToDevice)
                .map_or(0x02, |e| e.b_endpoint_address);

            let mut storage = UsbStorageDevice::new(new_addr, ctrl_idx, ep_in, ep_out, 0);
            if storage.query_inquiry_with_ctrl(ctrl).is_ok()
                && storage.query_capacity_with_ctrl(ctrl).is_ok()
            {
                klog_ok!(
                    "[usb] Bound USB Mass Storage driver: '{}' '{}' ({} sectors / {} MiB)",
                    storage.vendor,
                    storage.product,
                    storage.total_sectors,
                    (storage.total_sectors * (storage.sector_size as u64)) / (1024 * 1024)
                );
            }
            self.storage_devices.push(storage);
        }

        self.devices.push(usb_dev);
        Ok(())
    }

    /// Access controller mutably by index.
    pub fn get_controller_mut(&mut self, idx: usize) -> Option<&mut Box<dyn UsbHostController>> {
        self.controllers.get_mut(idx)
    }

    pub fn controllers(&self) -> &[Box<dyn UsbHostController>] {
        &self.controllers
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn devices(&self) -> &[UsbDevice] {
        &self.devices
    }

    pub fn find_device(&self, address: u8) -> Option<&UsbDevice> {
        self.devices.iter().find(|d| d.address == address)
    }

    pub fn keyboards(&self) -> &[UsbHidKeyboard] {
        &self.keyboards
    }

    pub fn keyboards_mut(&mut self) -> &mut [UsbHidKeyboard] {
        &mut self.keyboards
    }

    pub fn mice(&self) -> &[UsbHidMouse] {
        &self.mice
    }

    pub fn mice_mut(&mut self) -> &mut [UsbHidMouse] {
        &mut self.mice
    }

    pub fn storage_devices(&self) -> &[UsbStorageDevice] {
        &self.storage_devices
    }

    pub fn storage_devices_mut(&mut self) -> &mut [UsbStorageDevice] {
        &mut self.storage_devices
    }

    /// Polls interrupt endpoints for attached HID keyboards and mice.
    pub fn poll_hid(&mut self) {
        // Poll keyboards
        for kbd in &mut self.keyboards {
            let addr = kbd.device_address();
            let mut report = [0u8; 8];
            for ctrl in &mut self.controllers {
                if let Ok(len) = ctrl.interrupt_transfer(addr, 0x81, RequestDirection::DeviceToHost, &mut report) {
                    if len >= 8 {
                        kbd.process_report(&report);
                    }
                }
            }
        }

        // Poll mice
        for mouse in &mut self.mice {
            let addr = mouse.device_address();
            let mut report = [0u8; 4];
            for ctrl in &mut self.controllers {
                if let Ok(len) = ctrl.interrupt_transfer(addr, 0x82, RequestDirection::DeviceToHost, &mut report) {
                    if len >= 3 {
                        mouse.process_report(&report);
                    }
                }
            }
        }
    }
}

pub static USB_MANAGER: IrqSpinlock<UsbManager> = IrqSpinlock::new(UsbManager {
    controllers: Vec::new(),
    devices: Vec::new(),
    next_address: 1,
    keyboards: Vec::new(),
    mice: Vec::new(),
    storage_devices: Vec::new(),
});

/// Invokes a closure with mutable access to a specific host controller.
pub fn with_controller_mut<F, R>(idx: usize, f: F) -> Result<R, UsbError>
where
    F: FnOnce(&mut dyn UsbHostController) -> R,
{
    let mut mgr = USB_MANAGER.lock();
    if let Some(ctrl) = mgr.get_controller_mut(idx) {
        Ok(f(&mut **ctrl))
    } else {
        Err(UsbError::ControllerNotFound)
    }
}
