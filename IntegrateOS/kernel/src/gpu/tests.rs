//! Integrate OS — Phase 26 GPU Foundation Verification Test Suite
//!
//! Verifies:
//! - Test 1: PCI Display Controller identification (Vendor/Device ID, Class 0x03, VRAM & LFB)
//! - Test 2: GPU Device abstraction & hardware capability query
//! - Test 3: Display Mode configuration & Framebuffer baseline (1024x768x32)
//! - Test 4: GPU Surface management & offscreen render targets
//! - Test 5: 2D Command Stream execution, rasterization & fence synchronization

use crate::{klog_info, klog_ok};
use super::command::GpuCommand;
use super::manager::GPU_MANAGER;
use super::surface::SurfaceId;
use super::types::*;

/// Runs the full Phase 26 GPU test suite.
pub fn run_gpu_tests() {
    klog_info!("[gpu-test] Starting Phase 26 GPU Foundation verification...");

    test_1_pci_display_identification();
    test_2_device_abstraction_and_capabilities();
    test_3_display_mode_configuration();
    test_4_surface_management();
    test_5_command_stream_and_fences();

    klog_ok!("[gpu-test] ALL PHASE 26 GPU FOUNDATION TESTS PASSED (5/5)");
}

/// Test 1: PCI Display Controller identification and VRAM layout.
fn test_1_pci_display_identification() {
    klog_info!("[gpu-test] Test 1: PCI display controller identification...");

    let lock = GPU_MANAGER.lock();
    let mgr = lock.as_ref().expect("GPU_MANAGER not initialized");
    let dev = mgr.detected_device().expect("Test 1 Failed: No PCI Display Controller identified");

    // In VirtualBox, the display controller is 0x80EE:0xBEEF (or QEMU 0x1234:0x1111)
    klog_info!(
        "[gpu-test] Identified Display Device: {} (Vendor: 0x{:04X}, Device: 0x{:04X})",
        dev.model_name,
        dev.vendor_id,
        dev.device_id
    );

    assert!(
        dev.vendor_id == 0x80EE || dev.vendor_id == 0x1234 || dev.vendor_id == 0x8086 || dev.vendor_id == 0x15AD,
        "Test 1 Failed: Unexpected display vendor ID: 0x{:04X}",
        dev.vendor_id
    );

    let lfb_phys = dev.lfb_phys_address();
    assert!(lfb_phys != 0, "Test 1 Failed: LFB physical address is 0");
    assert!(dev.vram_size >= 1024 * 1024, "Test 1 Failed: VRAM size too small ({} bytes)", dev.vram_size);

    klog_ok!("[gpu-test] Test 1 PASSED: PCI display controller identification verified");
}

/// Test 2: GPU Device abstraction and capabilities.
fn test_2_device_abstraction_and_capabilities() {
    klog_info!("[gpu-test] Test 2: GPU device abstraction & capabilities...");

    let lock = GPU_MANAGER.lock();
    let mgr = lock.as_ref().expect("GPU_MANAGER not initialized");
    let driver = mgr.driver();

    let name = driver.name();
    assert!(!name.is_empty(), "Test 2 Failed: Empty driver name");

    let caps = driver.capabilities();
    assert!(caps.command_queue, "Test 2 Failed: Command queue capability missing");
    assert!(caps.vram_bytes > 0, "Test 2 Failed: 0 VRAM reported");

    let modes = driver.supported_modes();
    assert!(!modes.is_empty(), "Test 2 Failed: No supported display modes");

    let has_1024x768 = modes.iter().any(|m| m.width == 1024 && m.height == 768 && m.bpp == 32);
    assert!(has_1024x768, "Test 2 Failed: 1024x768x32 not in supported modes list");

    klog_ok!("[gpu-test] Test 2 PASSED: GPU device abstraction & capabilities verified");
}

/// Test 3: Display Mode configuration and Framebuffer baseline.
fn test_3_display_mode_configuration() {
    klog_info!("[gpu-test] Test 3: Display mode configuration & framebuffer baseline...");

    let mut lock = GPU_MANAGER.lock();
    let mgr = lock.as_mut().expect("GPU_MANAGER not initialized");

    // 1. Verify current default mode is 1024x768x32
    let cur_mode = mgr.driver().current_mode();
    assert_eq!(cur_mode.width, 1024);
    assert_eq!(cur_mode.height, 768);
    assert_eq!(cur_mode.bpp, 32);
    assert_eq!(cur_mode.pitch(), 4096);
    assert_eq!(cur_mode.framebuffer_size(), 1024 * 768 * 4);

    let (ar_w, ar_h) = cur_mode.aspect_ratio();
    assert_eq!((ar_w, ar_h), (4, 3));

    // 2. Test mode switch to 800x600x32
    let mode_800 = DisplayMode::MODE_800X600;
    let set_res = mgr.driver_mut().set_mode(mode_800);
    assert!(set_res.is_ok(), "Test 3 Failed: set_mode(800x600) failed");

    let new_mode = mgr.driver().current_mode();
    assert_eq!(new_mode.width, 800);
    assert_eq!(new_mode.height, 600);
    assert_eq!(new_mode.pitch(), 3200);

    // 3. Restore mode back to 1024x768x32
    let mode_1024 = DisplayMode::MODE_1024X768;
    let restore_res = mgr.driver_mut().set_mode(mode_1024);
    assert!(restore_res.is_ok(), "Test 3 Failed: set_mode(1024x768) failed");

    let restored_mode = mgr.driver().current_mode();
    assert_eq!(restored_mode.width, 1024);
    assert_eq!(restored_mode.height, 768);

    // 4. Verify FramebufferInfo
    let fb_info = mgr.driver().framebuffer_info();
    assert!(fb_info.phys_addr != 0);
    assert_eq!(fb_info.width, 1024);
    assert_eq!(fb_info.height, 768);
    assert_eq!(fb_info.pitch, 4096);
    assert_eq!(fb_info.bpp, 32);

    klog_ok!("[gpu-test] Test 3 PASSED: Display mode configuration & framebuffer baseline verified");
}

/// Test 4: GPU Surface management and offscreen render targets.
fn test_4_surface_management() {
    klog_info!("[gpu-test] Test 4: GPU surface management & offscreen render targets...");

    let mut lock = GPU_MANAGER.lock();
    let mgr = lock.as_mut().expect("GPU_MANAGER not initialized");
    let driver = mgr.driver_mut();

    // 1. Create a 128x128 32-bit offscreen surface
    let surf_id = driver
        .create_surface(128, 128, PixelFormat::Bgra8888)
        .expect("Test 4 Failed: create_surface failed");
    assert!(surf_id != SurfaceId::SCANOUT);

    // 2. Manipulate pixels and test boundary checks
    {
        let surf = driver.get_surface_mut(surf_id).expect("Test 4 Failed: surface not found");
        assert_eq!(surf.width, 128);
        assert_eq!(surf.height, 128);
        assert_eq!(surf.pitch, 512);

        surf.set_pixel(10, 20, 0x11223344);
        assert_eq!(surf.get_pixel(10, 20), 0x11223344);

        // Out of bounds check
        surf.set_pixel(200, 200, 0xAABBCCDD);
        assert_eq!(surf.get_pixel(200, 200), 0);

        // Fill rect
        surf.fill_rect(5, 5, 20, 10, 0x55667788);
        assert_eq!(surf.get_pixel(10, 10), 0x55667788);
        assert_eq!(surf.get_pixel(4, 4), 0);
    }

    // 3. Destroy surface
    let destroy_res = driver.destroy_surface(surf_id);
    assert!(destroy_res.is_ok(), "Test 4 Failed: destroy_surface failed");

    // 4. Verify surface is no longer found
    assert!(driver.get_surface(surf_id).is_none(), "Test 4 Failed: surface still exists after destroy");

    klog_ok!("[gpu-test] Test 4 PASSED: GPU surface management & offscreen targets verified");
}

/// Test 5: 2D Command Stream execution, rasterization & fence synchronization.
fn test_5_command_stream_and_fences() {
    klog_info!("[gpu-test] Test 5: 2D command stream execution & fence synchronization...");

    let mut lock = GPU_MANAGER.lock();
    let mgr = lock.as_mut().expect("GPU_MANAGER not initialized");
    let driver = mgr.driver_mut();

    // 1. Create offscreen render target (200x200)
    let surf_id = driver
        .create_surface(200, 200, PixelFormat::Bgra8888)
        .expect("Test 5 Failed: create_surface failed");

    // 2. Submit a sequence of 2D rendering commands
    driver
        .submit_command(GpuCommand::Clear {
            surface: surf_id,
            color: 0xFF102030,
        })
        .unwrap();

    driver
        .submit_command(GpuCommand::FillRect {
            surface: surf_id,
            x: 20,
            y: 30,
            width: 50,
            height: 40,
            color: 0xFFAABBCC,
        })
        .unwrap();

    driver
        .submit_command(GpuCommand::DrawLine {
            surface: surf_id,
            x0: 0,
            y0: 0,
            x1: 50,
            y1: 50,
            color: 0xFFFFFFFF,
        })
        .unwrap();

    driver.submit_command(GpuCommand::Fence { fence_id: 101 }).unwrap();

    // 3. Flush commands and wait for fence
    let wait_res = driver.wait_fence(101);
    assert!(wait_res.is_ok(), "Test 5 Failed: wait_fence returned error: {:?}", wait_res);

    // 4. Verify pixel rasterization results
    let surf = driver.get_surface(surf_id).expect("Test 5 Failed: target surface missing");

    // Pixel inside rectangle (x: 30, y: 40)
    let rect_pixel = surf.get_pixel(30, 40);
    assert_eq!(
        rect_pixel, 0xFFAABBCC,
        "Test 5 Failed: FillRect pixel at (30, 40) expected 0xFFAABBCC, got 0x{:08X}",
        rect_pixel
    );

    // Pixel along diagonal line (x: 15, y: 15)
    let line_pixel = surf.get_pixel(15, 15);
    assert_eq!(
        line_pixel, 0xFFFFFFFF,
        "Test 5 Failed: DrawLine pixel at (15, 15) expected 0xFFFFFFFF, got 0x{:08X}",
        line_pixel
    );

    // Background pixel outside rectangle and line (x: 150, y: 150)
    let bg_pixel = surf.get_pixel(150, 150);
    assert_eq!(
        bg_pixel, 0xFF102030,
        "Test 5 Failed: Clear background pixel at (150, 150) expected 0xFF102030, got 0x{:08X}",
        bg_pixel
    );

    // 5. Clean up surface
    let _ = driver.destroy_surface(surf_id);

    klog_ok!("[gpu-test] Test 5 PASSED: 2D command stream execution & fence synchronization verified");
}
