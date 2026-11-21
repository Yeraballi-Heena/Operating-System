//! Integrate OS — Phase 25 USB Verification Test Suite
//!
//! Verifies:
//! - Test 1: Host Controller discovery, capability registers, and driver abstraction
//! - Test 2: Standard Setup packet serialization, descriptor parsing & UTF-16 strings
//! - Test 3: USB bus enumeration, address assignment, and configuration activation
//! - Test 4: USB HID Boot Keyboard and Mouse drivers & input queue event routing
//! - Test 5: USB Mass Storage BOT (Bulk-Only Transport) protocol & SCSI block operations

use crate::arch::x86_64::keyboard::KEYBOARD_QUEUE;
use crate::drivers::device::{Driver, StorageDriver};
use crate::input::pop_event;
use crate::{klog_info, klog_ok};
use super::controller::UsbControllerType;
use super::descriptors::*;
use super::manager::USB_MANAGER;
use super::msc::bot::{CommandBlockWrapper, CommandStatusWrapper, CBW_SIGNATURE, CSW_SIGNATURE};
use super::msc::scsi::*;
use super::setup::UsbSetupPacket;
use super::types::*;

/// Runs the full Phase 25 USB test suite.
pub fn run_usb_tests() {
    klog_info!("[usb-test] Starting Phase 25 USB verification...");

    test_1_host_controllers();
    test_2_setup_and_descriptors();
    test_3_bus_enumeration();
    test_4_hid_drivers();
    test_5_mass_storage();

    klog_ok!("[usb-test] ALL PHASE 25 USB TESTS PASSED (5/5)");
}

/// Test 1: Host Controller discovery and abstraction.
fn test_1_host_controllers() {
    klog_info!("[usb-test] Test 1: Host controller discovery & controller abstraction...");

    let mgr = USB_MANAGER.lock();
    assert!(
        !mgr.devices().is_empty(),
        "Test 1 Failed: No USB devices or host controllers registered"
    );

    // Verify UsbSpeed strings
    assert_eq!(UsbSpeed::LowSpeed.as_str(), "Low-Speed (1.5 Mbps)");
    assert_eq!(UsbSpeed::FullSpeed.as_str(), "Full-Speed (12 Mbps)");
    assert_eq!(UsbSpeed::HighSpeed.as_str(), "High-Speed (480 Mbps)");
    assert_eq!(UsbSpeed::SuperSpeed.as_str(), "SuperSpeed (5 Gbps)");

    // Verify UsbControllerType strings
    assert_eq!(UsbControllerType::Uhci.as_str(), "UHCI (USB 1.1)");
    assert_eq!(UsbControllerType::Ehci.as_str(), "EHCI (USB 2.0)");
    assert_eq!(UsbControllerType::Xhci.as_str(), "xHCI (USB 3.0+)");

    // Verify error string mappings
    assert_eq!(UsbError::Timeout.as_str(), "USB transfer timed out");
    assert_eq!(UsbError::Stall.as_str(), "USB pipe stalled (STALL handshake received)");

    klog_ok!("[usb-test] Test 1 PASSED: Host controller discovery & abstraction verified");
}

/// Test 2: Standard Setup packet serialization and descriptor parsing.
fn test_2_setup_and_descriptors() {
    klog_info!("[usb-test] Test 2: Standard Setup packets & descriptor parsing...");

    // 1. Setup Packet verification
    let setup_get_desc = UsbSetupPacket::get_descriptor(DESC_DEVICE, 0, 18);
    let bytes = setup_get_desc.to_bytes();
    assert_eq!(bytes.len(), 8);
    assert_eq!(bytes[0], 0x80); // Device-to-Host, Standard, Device
    assert_eq!(bytes[1], REQ_GET_DESCRIPTOR);
    assert_eq!(setup_get_desc.direction(), RequestDirection::DeviceToHost);
    assert_eq!(setup_get_desc.request_type(), RequestType::Standard);
    assert_eq!(setup_get_desc.recipient(), RequestRecipient::Device);

    let parsed_setup = UsbSetupPacket::from_bytes(&bytes).expect("Failed to parse setup packet");
    assert_eq!(parsed_setup.bm_request_type, 0x80);
    assert_eq!(parsed_setup.b_request, REQ_GET_DESCRIPTOR);
    let setup_w_len = parsed_setup.w_length;
    assert_eq!(setup_w_len, 18);

    let setup_set_addr = UsbSetupPacket::set_address(42);
    let setup_w_val = setup_set_addr.w_value;
    assert_eq!(setup_w_val, 42);
    assert_eq!(setup_set_addr.b_request, REQ_SET_ADDRESS);
    assert_eq!(setup_set_addr.direction(), RequestDirection::HostToDevice);

    // 2. Device Descriptor verification
    let dev_desc = UsbDeviceDescriptor {
        b_length: 18,
        b_descriptor_type: DESC_DEVICE,
        bcd_usb: 0x0200,
        b_device_class: CLASS_PER_INTERFACE,
        b_device_subclass: 0,
        b_device_protocol: 0,
        b_max_packet_size0: 64,
        id_vendor: 0x8086,
        id_product: 0x1234,
        bcd_device: 0x0100,
        i_manufacturer: 1,
        i_product: 2,
        i_serial_number: 3,
        b_num_configurations: 1,
    };
    let dev_bytes = dev_desc.to_bytes();
    assert_eq!(dev_bytes.len(), 18);
    let parsed_dev = UsbDeviceDescriptor::from_bytes(&dev_bytes).expect("Failed to parse dev desc");
    assert_eq!(parsed_dev.id_vendor, 0x8086);
    assert_eq!(parsed_dev.id_product, 0x1234);
    assert_eq!(parsed_dev.bcd_usb, 0x0200);

    // 3. Configuration Descriptor verification
    let config_desc = UsbConfigurationDescriptor {
        b_length: 9,
        b_descriptor_type: DESC_CONFIGURATION,
        w_total_length: 34,
        b_num_interfaces: 1,
        b_configuration_value: 1,
        i_configuration: 0,
        bm_attributes: 0xE0, // Self-powered + remote wakeup
        b_max_power: 50,     // 100 mA
    };
    let config_bytes = config_desc.to_bytes();
    let parsed_config = UsbConfigurationDescriptor::from_bytes(&config_bytes).expect("Parse config");
    assert_eq!(parsed_config.w_total_length, 34);
    assert!(parsed_config.is_self_powered());
    assert!(parsed_config.supports_remote_wakeup());
    assert_eq!(parsed_config.max_power_ma(), 100);

    // 4. Endpoint Descriptor verification
    let ep_desc = UsbEndpointDescriptor {
        b_length: 7,
        b_descriptor_type: DESC_ENDPOINT,
        b_endpoint_address: 0x81, // IN Endpoint 1
        bm_attributes: 0x02,       // Bulk
        w_max_packet_size: 512,
        b_interval: 0,
    };
    assert_eq!(ep_desc.endpoint_number(), 1);
    assert_eq!(ep_desc.direction(), RequestDirection::DeviceToHost);
    assert_eq!(ep_desc.transfer_type(), UsbTransferType::Bulk);

    // 5. String Descriptor decoding
    let raw_string = [
        18, DESC_STRING,
        b'I', 0, b'n', 0, b't', 0, b'e', 0, b'g', 0, b'r', 0, b'a', 0, b't', 0,
    ];
    let decoded_str = parse_string_descriptor(&raw_string).expect("Parse string descriptor");
    assert_eq!(decoded_str, "Integrat");

    klog_ok!("[usb-test] Test 2 PASSED: Setup packets & descriptor parsing verified");
}

/// Test 3: USB Bus enumeration and device registry.
fn test_3_bus_enumeration() {
    klog_info!("[usb-test] Test 3: USB bus enumeration & topology...");

    let mgr = USB_MANAGER.lock();
    let dev_count = mgr.device_count();
    assert!(dev_count >= 3, "Test 3 Failed: Expected >= 3 enumerated devices, got {}", dev_count);

    // Verify presence of Keyboard, Mouse, and Mass Storage
    let mut found_kbd = false;
    let mut found_mouse = false;
    let mut found_msc = false;

    for dev in mgr.devices() {
        if dev.is_hid_keyboard() {
            found_kbd = true;
            assert_eq!(dev.vendor_id(), 0x1234);
            assert_eq!(dev.product_id(), 0x0001);
            assert_eq!(dev.current_configuration, 1);
        } else if dev.is_hid_mouse() {
            found_mouse = true;
            assert_eq!(dev.vendor_id(), 0x1234);
            assert_eq!(dev.product_id(), 0x0002);
            assert_eq!(dev.current_configuration, 1);
        } else if dev.is_mass_storage() {
            found_msc = true;
            assert_eq!(dev.vendor_id(), 0xABCD);
            assert_eq!(dev.product_id(), 0x1234);
            assert_eq!(dev.current_configuration, 1);
        }
    }

    assert!(found_kbd, "Test 3 Failed: USB HID Keyboard not enumerated");
    assert!(found_mouse, "Test 3 Failed: USB HID Mouse not enumerated");
    assert!(found_msc, "Test 3 Failed: USB Mass Storage device not enumerated");

    klog_ok!("[usb-test] Test 3 PASSED: USB bus enumeration & device registry verified");
}

/// Test 4: USB HID Boot Keyboard and Mouse drivers.
fn test_4_hid_drivers() {
    klog_info!("[usb-test] Test 4: USB HID Boot Keyboard & Mouse drivers...");

    // Clear any leftover keyboard queue entries
    while KEYBOARD_QUEUE.lock().pop().is_some() {}

    // 1. Test Keyboard Report: Press 'A' (Usage 0x04) without modifiers
    let report_a = [0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00];
    {
        let mut mgr = USB_MANAGER.lock();
        if let Some(kbd) = mgr.keyboards_mut().first_mut() {
            kbd.process_report(&report_a);
        }
    }

    let ch = KEYBOARD_QUEUE.lock().pop();
    assert_eq!(ch, Some(b'a'), "Test 4 Failed: Expected 'a' in keyboard queue, got {:?}", ch);

    // Verify InputEvent was pushed
    let mut event_found = false;
    while let Some(ev) = pop_event() {
        if let crate::input::events::InputEventKind::Key(k) = ev.kind {
            if k.code == crate::input::events::KeyCode::A {
                event_found = true;
                break;
            }
        }
    }
    assert!(event_found, "Test 4 Failed: KeyCode::A not dispatched to input queue");

    // 2. Test Keyboard Report: Shift + 'B' (Usage 0x05)
    let report_b = [0x02, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00]; // 0x02 = Left Shift
    {
        let mut mgr = USB_MANAGER.lock();
        if let Some(kbd) = mgr.keyboards_mut().first_mut() {
            kbd.process_report(&report_b);
        }
    }
    let ch_b = KEYBOARD_QUEUE.lock().pop();
    assert_eq!(ch_b, Some(b'B'), "Test 4 Failed: Expected 'B' in keyboard queue, got {:?}", ch_b);

    // 3. Test Mouse Report: dx = +20, dy = -10, Left Button pressed (0x01)
    let mouse_report = [0x01, 20, (-10i8) as u8, 0];
    {
        let mut mgr = USB_MANAGER.lock();
        if let Some(mouse) = mgr.mice_mut().first_mut() {
            mouse.process_report(&mouse_report);
        }
    }

    let mut mouse_event_found = false;
    while let Some(ev) = pop_event() {
        if let crate::input::events::InputEventKind::Mouse(m) = ev.kind {
            if m.delta == (20, -10) && m.buttons.is_left() {
                mouse_event_found = true;
                break;
            }
        }
    }
    assert!(mouse_event_found, "Test 4 Failed: Mouse delta & button not dispatched");

    klog_ok!("[usb-test] Test 4 PASSED: USB HID Boot Keyboard & Mouse drivers verified");
}

/// Test 5: USB Mass Storage BOT and SCSI Block Operations.
fn test_5_mass_storage() {
    klog_info!("[usb-test] Test 5: USB Mass Storage BOT & SCSI block operations...");

    // 1. Verify CBW and CSW structures
    let cdb_inquiry = build_inquiry_cdb(36);
    let cbw = CommandBlockWrapper::new(0x11223344, 36, RequestDirection::DeviceToHost, 0, &cdb_inquiry);
    assert_eq!(cbw.d_cbw_signature, CBW_SIGNATURE);
    assert_eq!(cbw.d_cbw_tag, 0x11223344);
    assert_eq!(cbw.d_cbw_data_transfer_length, 36);
    assert_eq!(cbw.bm_cbw_flags, 0x80);

    let cbw_bytes = cbw.to_bytes();
    let parsed_cbw = CommandBlockWrapper::from_bytes(&cbw_bytes).expect("Parse CBW");
    assert_eq!(parsed_cbw.d_cbw_tag, 0x11223344);

    let csw = CommandStatusWrapper::new(0x11223344, 0, 0);
    assert_eq!(csw.d_csw_signature, CSW_SIGNATURE);
    assert!(csw.is_success());
    let csw_bytes = csw.to_bytes();
    let parsed_csw = CommandStatusWrapper::from_bytes(&csw_bytes).expect("Parse CSW");
    assert!(parsed_csw.is_success());

    // 2. Query Storage Driver registered in USB_MANAGER
    let (dev_addr, ctrl_id, ep_in, ep_out, lun) = {
        let mgr = USB_MANAGER.lock();
        assert!(
            !mgr.storage_devices().is_empty(),
            "Test 5 Failed: No USB Storage device bound"
        );
        let s = &mgr.storage_devices()[0];
        assert_eq!(s.block_size(), 512);
        assert!(s.total_blocks() > 0, "Test 5 Failed: Total blocks is 0");
        assert_eq!(s.vendor, "Integrat");
        assert_eq!(s.product, "Flash Drive");
        (s.dev_address, s.controller_id, s.ep_in, s.ep_out, s.lun)
    };

    // 3. Test Block Write and Read via StorageDriver with lock dropped
    let mut storage = super::msc::UsbStorageDevice::new(dev_addr, ctrl_id, ep_in, ep_out, lun);
    let init_res = storage.init();
    assert!(init_res.is_ok(), "Test 5 Failed: storage.init failed: {:?}", init_res);

    let mut write_buf = [0u8; 512];
    for (i, byte) in write_buf.iter_mut().enumerate() {
        *byte = ((i * 7 + 0x42) & 0xFF) as u8;
    }

    // Write to Sector 5
    let write_res = storage.write_block(5, &write_buf);
    assert!(write_res.is_ok(), "Test 5 Failed: write_block returned error: {:?}", write_res);

    // Read back from Sector 5
    let mut read_buf = [0u8; 512];
    let read_res = storage.read_block(5, &mut read_buf);
    assert!(read_res.is_ok(), "Test 5 Failed: read_block returned error: {:?}", read_res);

    assert_eq!(
        &write_buf[..],
        &read_buf[..],
        "Test 5 Failed: Block readback data does not match written pattern"
    );

    klog_ok!("[usb-test] Test 5 PASSED: USB Mass Storage BOT & SCSI block operations verified");
}
