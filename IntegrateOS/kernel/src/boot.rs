//! Integrate OS — GRUB Multiboot Header & 32->64-Bit Long Mode Bootstrap
//!
//! Preserves the conceptual boot flow of *The Little Book About Operating System Development*:
//! Firmware / BIOS -> GRUB -> Multiboot1 (0x1BADB002) -> `_start` (32-bit Protected Mode)
//! -> 4-Level Identity Paging + Long Mode Transition -> `kernel_main` (64-bit Rust).

use core::arch::global_asm;

/// Multiboot1 header magic constant required by GRUB (`0x1BADB002`).
pub const MULTIBOOT_HEADER_MAGIC: u32 = 0x1BAD_B002;

/// Multiboot1 bootloader handoff magic passed in `EAX` (`0x2BADB002`).
pub const MULTIBOOT_BOOTLOADER_MAGIC: u32 = 0x2BAD_B002;

/// Multiboot1 header flags:
/// - Bit 0 (`0x0000_0001`): Align boot modules on 4 KiB page boundaries.
/// - Bit 1 (`0x0000_0002`): Provide memory map information (`mem_*` / `mmap_*`).
pub const MULTIBOOT_HEADER_FLAGS: u32 = 0x0000_0003;

/// Multiboot1 checksum such that `magic + flags + checksum == 0` (mod 2^32).
pub const MULTIBOOT_HEADER_CHECKSUM: u32 =
    0u32.wrapping_sub(MULTIBOOT_HEADER_MAGIC.wrapping_add(MULTIBOOT_HEADER_FLAGS));

/// Standard 12-byte Multiboot1 header placed in the `.multiboot` section.
#[repr(C, align(4))]
pub struct MultibootHeader {
    pub magic: u32,
    pub flags: u32,
    pub checksum: u32,
}

#[used]
#[link_section = ".multiboot"]
pub static MULTIBOOT_HEADER: MultibootHeader = MultibootHeader {
    magic: MULTIBOOT_HEADER_MAGIC,
    flags: MULTIBOOT_HEADER_FLAGS,
    checksum: MULTIBOOT_HEADER_CHECKSUM,
};

/// Subset of the Multiboot1 Information structure (`multiboot_info_t`) passed by GRUB in `EBX`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MultibootInfo {
    pub flags: u32,
    pub mem_lower: u32,
    pub mem_upper: u32,
    pub boot_device: u32,
    pub cmdline: u32,
    pub mods_count: u32,
    pub mods_addr: u32,
    pub syms: [u32; 4],
    pub mmap_length: u32,
    pub mmap_addr: u32,
    pub drives_length: u32,
    pub drives_addr: u32,
    pub config_table: u32,
    pub boot_loader_name: u32,
}

impl MultibootInfo {
    /// Reads the Multiboot information structure from the physical address provided by GRUB.
    ///
    /// # Safety
    /// Caller must ensure `phys_addr` points to a valid identity-mapped `MultibootInfo` structure.
    pub unsafe fn from_phys_addr(phys_addr: u64) -> Option<Self> {
        if phys_addr == 0 {
            return None;
        }
        let ptr = phys_addr as *const MultibootInfo;
        // SAFETY: Identity-mapped in the first 4 GiB by bootstrap page tables.
        Some(unsafe { core::ptr::read_volatile(ptr) })
    }

    /// Returns the bootloader name string if bit 9 of `flags` is set.
    pub fn bootloader_name(&self) -> &'static str {
        const FLAG_BOOT_LOADER_NAME: u32 = 1 << 9;
        if (self.flags & FLAG_BOOT_LOADER_NAME) == 0 || self.boot_loader_name == 0 {
            return "GRUB (Multiboot)";
        }
        let ptr = self.boot_loader_name as *const u8;
        let mut len = 0usize;
        while len < 64 {
            // SAFETY: Identity-mapped bootloader string in low memory.
            let b = unsafe { core::ptr::read_volatile(ptr.add(len)) };
            if b == 0 {
                break;
            }
            len += 1;
        }
        // SAFETY: Validated length up to NUL terminator in static low memory.
        let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
        core::str::from_utf8(slice).unwrap_or("GRUB (Multiboot)")
    }
}

/// 4 KiB page table buffer used during bootstrap identity mapping.
#[repr(C, align(4096))]
pub struct PageTableBuf([u64; 512]);

#[no_mangle]
#[link_section = ".bss.page_tables"]
pub static mut boot_pml4: PageTableBuf = PageTableBuf([0; 512]);

#[no_mangle]
#[link_section = ".bss.page_tables"]
pub static mut boot_pdpt: PageTableBuf = PageTableBuf([0; 512]);

#[no_mangle]
#[link_section = ".bss.page_tables"]
pub static mut boot_pd: [PageTableBuf; 4] = [
    PageTableBuf([0; 512]),
    PageTableBuf([0; 512]),
    PageTableBuf([0; 512]),
    PageTableBuf([0; 512]),
];

/// 64 KiB kernel bootstrap stack in `.bss.stack`.
pub const KERNEL_STACK_SIZE: usize = 64 * 1024;

#[repr(C, align(16))]
pub struct KernelStack([u8; KERNEL_STACK_SIZE]);

#[used]
#[no_mangle]
#[link_section = ".bss.stack"]
pub static mut KERNEL_BOOT_STACK: KernelStack = KernelStack([0; KERNEL_STACK_SIZE]);

global_asm!(
    r#"
    .section .text.boot, "ax", @progbits
    .global _start
    .code32
_start:
    cli
    cld

    // Set initial 32-bit stack pointer to top of 64 KiB kernel stack
    mov esp, offset __stack_top

    // Preserve Multiboot1 handoff registers:
    // EDI = EAX (Multiboot magic 0x2BADB002)
    // ESI = EBX (Multiboot info physical address)
    mov edi, eax
    mov esi, ebx

    // Zero bootstrap page tables (boot_pml4, boot_pdpt, boot_pd[0..4] = 6 * 4096 bytes)
    push edi
    mov edi, offset boot_pml4
    mov ecx, 6144
    xor eax, eax
    rep stosd
    pop edi

    // Map boot_pml4[0] -> boot_pdpt (PRESENT | WRITABLE = 0x03)
    mov eax, offset boot_pdpt
    or eax, 0x03
    mov dword ptr [boot_pml4], eax

    // Map boot_pdpt[0..4] -> boot_pd[0..4] (4 GiB total identity map)
    mov eax, offset boot_pd
    or eax, 0x03
    mov dword ptr [boot_pdpt + 0], eax
    add eax, 0x1000
    mov dword ptr [boot_pdpt + 8], eax
    add eax, 0x1000
    mov dword ptr [boot_pdpt + 16], eax
    add eax, 0x1000
    mov dword ptr [boot_pdpt + 24], eax

    // Fill 2048 entries across boot_pd[0..4] with 2 MiB huge pages (0x83 = P | RW | PS)
    mov ebx, offset boot_pd
    mov eax, 0x00000083
    xor edx, edx
    mov ecx, 2048
2:
    mov dword ptr [ebx], eax
    mov dword ptr [ebx + 4], edx
    add eax, 0x00200000
    adc edx, 0
    add ebx, 8
    dec ecx
    jnz 2b

    // Load PML4 physical address into CR3
    mov eax, offset boot_pml4
    mov cr3, eax

    // Enable PAE (CR4 bit 5)
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    // Enable Long Mode (IA32_EFER.LME = bit 8)
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    // Enable Paging (CR0.PG = bit 31) and Write Protect (CR0.WP = bit 16)
    mov eax, cr0
    or eax, (1 << 31) | (1 << 16)
    mov cr0, eax

    // Load 64-bit bootstrap GDT and transition to 64-bit Long Mode
    lgdt [boot_gdtr]
    push 0x08
    mov eax, offset long_mode_start
    push eax
    retf

    .code64
long_mode_start:
    // Load 64-bit kernel data segment (0x10) into all data segment registers
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    // Initialize 64-bit stack pointer and frame pointer
    lea rsp, [rip + __stack_top]
    xor rbp, rbp

    // Zero-extend EDI -> RDI (multiboot_magic) and ESI -> RSI (multiboot_info_addr)
    mov edi, edi
    mov esi, esi

    call kernel_main

3:
    cli
    hlt
    jmp 3b

    .align 16
boot_gdt:
    .quad 0x0000000000000000 // 0x00: Null descriptor
    .quad 0x00AF9A000000FFFF // 0x08: 64-bit Kernel Code (L=1, D=0, P=1, DPL=0, RX)
    .quad 0x00CF92000000FFFF // 0x10: 64-bit Kernel Data (P=1, DPL=0, RW)
boot_gdt_end:

boot_gdtr:
    .word boot_gdt_end - boot_gdt - 1
    .quad boot_gdt
"#
);
