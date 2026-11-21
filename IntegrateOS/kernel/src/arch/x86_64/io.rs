//! x86_64 Port I/O Instructions & PCI Configuration Access
//!
//! Provides direct hardware port I/O (`inb`/`outb`, `inw`/`outw`, `inl`/`outl`)
//! and PCI Type 1 Configuration Space access (`0xCF8` / `0xCFC`).

pub const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
pub const PCI_CONFIG_DATA: u16 = 0xCFC;

/// Writes an 8-bit byte `val` to x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is valid and the operation preserves hardware/memory safety.
#[inline(always)]
pub unsafe fn outb(port: u16, val: u8) {
    // SAFETY: Direct Ring 0 port I/O.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Reads an 8-bit byte from x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is valid.
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    // SAFETY: Direct Ring 0 port I/O.
    unsafe {
        core::arch::asm!(
            "in al, dx",
            out("al") val,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    val
}

/// Writes a 16-bit word `val` to x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is valid.
#[inline(always)]
pub unsafe fn outw(port: u16, val: u16) {
    // SAFETY: Direct Ring 0 port I/O.
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") val,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Reads a 16-bit word from x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is valid.
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let val: u16;
    // SAFETY: Direct Ring 0 port I/O.
    unsafe {
        core::arch::asm!(
            "in ax, dx",
            out("ax") val,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    val
}

/// Writes a 32-bit dword `val` to x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is valid.
#[inline(always)]
pub unsafe fn outl(port: u16, val: u32) {
    // SAFETY: Direct Ring 0 port I/O.
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") val,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Reads a 32-bit dword from x86 I/O `port`.
///
/// # Safety
/// Caller must ensure `port` is valid.
#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
    let val: u32;
    // SAFETY: Direct Ring 0 port I/O.
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            out("eax") val,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    val
}

/// Brief I/O delay (writes 0 to legacy diagnostic port 0x80).
#[inline(always)]
pub fn io_wait() {
    // SAFETY: Writing to port 0x80 is standard safe post-code delay on x86.
    unsafe {
        outb(0x80, 0x00);
    }
}

/// Reads a 32-bit configuration dword from PCI Configuration Mechanism #1.
///
/// # Safety
/// Caller must ensure valid bus/device/function coordinates.
pub unsafe fn pci_read_u32(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let address = (1u32 << 31)
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC);
    // SAFETY: Accessing standard x86 PCI configuration ports 0xCF8 and 0xCFC.
    unsafe {
        outl(PCI_CONFIG_ADDRESS, address);
        inl(PCI_CONFIG_DATA)
    }
}

/// Writes a 32-bit configuration dword to PCI Configuration Mechanism #1.
///
/// # Safety
/// Caller must ensure valid bus/device/function coordinates and register write safety.
pub unsafe fn pci_write_u32(bus: u8, dev: u8, func: u8, offset: u8, val: u32) {
    let address = (1u32 << 31)
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC);
    // SAFETY: Accessing standard x86 PCI configuration ports 0xCF8 and 0xCFC.
    unsafe {
        outl(PCI_CONFIG_ADDRESS, address);
        outl(PCI_CONFIG_DATA, val);
    }
}
