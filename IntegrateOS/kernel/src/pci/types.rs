//! Integrate OS — PCI Configuration Space Registers & Constants
//!
//! Provides standard register offsets, command flags, and status flags for PCI 2.3 and PCIe.

// Standard Configuration Offsets (Type 0x00 and Type 0x01)
pub const PCI_VENDOR_ID: u8 = 0x00;
pub const PCI_DEVICE_ID: u8 = 0x02;
pub const PCI_COMMAND: u8 = 0x04;
pub const PCI_STATUS: u8 = 0x06;
pub const PCI_REVISION_ID: u8 = 0x08;
pub const PCI_PROG_IF: u8 = 0x09;
pub const PCI_SUBCLASS: u8 = 0x0A;
pub const PCI_CLASS: u8 = 0x0B;
pub const PCI_CACHE_LINE_SIZE: u8 = 0x0C;
pub const PCI_LATENCY_TIMER: u8 = 0x0D;
pub const PCI_HEADER_TYPE: u8 = 0x0E;
pub const PCI_BIST: u8 = 0x0F;
pub const PCI_BAR0: u8 = 0x10;
pub const PCI_BAR1: u8 = 0x14;
pub const PCI_BAR2: u8 = 0x18;
pub const PCI_BAR3: u8 = 0x1C;
pub const PCI_BAR4: u8 = 0x20;
pub const PCI_BAR5: u8 = 0x24;
pub const PCI_CARDBUS_CIS_PTR: u8 = 0x28;
pub const PCI_SUBSYSTEM_VENDOR_ID: u8 = 0x2C;
pub const PCI_SUBSYSTEM_ID: u8 = 0x2E;
pub const PCI_EXPANSION_ROM_BASE: u8 = 0x30;
pub const PCI_CAPABILITIES_PTR: u8 = 0x34;
pub const PCI_INTERRUPT_LINE: u8 = 0x3C;
pub const PCI_INTERRUPT_PIN: u8 = 0x3D;
pub const PCI_MIN_GNT: u8 = 0x3E;
pub const PCI_MAX_LAT: u8 = 0x3F;

// PCI-to-PCI Bridge Specific Registers (Type 0x01)
pub const PCI_PRIMARY_BUS: u8 = 0x18;
pub const PCI_SECONDARY_BUS: u8 = 0x19;
pub const PCI_SUBORDINATE_BUS: u8 = 0x1A;

// Command Register Bitflags
pub const PCI_COMMAND_IO_SPACE: u16 = 1 << 0;
pub const PCI_COMMAND_MEMORY_SPACE: u16 = 1 << 1;
pub const PCI_COMMAND_BUS_MASTER: u16 = 1 << 2;
pub const PCI_COMMAND_SPECIAL_CYCLES: u16 = 1 << 3;
pub const PCI_COMMAND_MEM_WRITE_INVALIDATE: u16 = 1 << 4;
pub const PCI_COMMAND_VGA_PALETTE_SNOOP: u16 = 1 << 5;
pub const PCI_COMMAND_PARITY_ERROR_RESPONSE: u16 = 1 << 6;
pub const PCI_COMMAND_SERR_ENABLE: u16 = 1 << 8;
pub const PCI_COMMAND_FAST_BACK_TO_BACK: u16 = 1 << 9;
pub const PCI_COMMAND_INTERRUPT_DISABLE: u16 = 1 << 10;

// Status Register Bitflags
pub const PCI_STATUS_INTERRUPT_STATUS: u16 = 1 << 3;
pub const PCI_STATUS_CAPABILITIES_LIST: u16 = 1 << 4;
pub const PCI_STATUS_66MHZ_CAPABLE: u16 = 1 << 5;
pub const PCI_STATUS_FAST_BACK_TO_BACK: u16 = 1 << 7;
pub const PCI_STATUS_MASTER_DATA_PARITY_ERROR: u16 = 1 << 8;
pub const PCI_STATUS_DEVSEL_TIMING_MASK: u16 = 3 << 9;
pub const PCI_STATUS_SIGNALED_TARGET_ABORT: u16 = 1 << 11;
pub const PCI_STATUS_RECEIVED_TARGET_ABORT: u16 = 1 << 12;
pub const PCI_STATUS_RECEIVED_MASTER_ABORT: u16 = 1 << 13;
pub const PCI_STATUS_SIGNALED_SYSTEM_ERROR: u16 = 1 << 14;
pub const PCI_STATUS_DETECTED_PARITY_ERROR: u16 = 1 << 15;

// Header Types
pub const PCI_HEADER_TYPE_DEVICE: u8 = 0x00;
pub const PCI_HEADER_TYPE_BRIDGE: u8 = 0x01;
pub const PCI_HEADER_TYPE_CARDBUS: u8 = 0x02;
pub const PCI_HEADER_TYPE_MULTIFUNCTION_MASK: u8 = 0x80;
