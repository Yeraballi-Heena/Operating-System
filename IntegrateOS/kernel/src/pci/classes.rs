//! Integrate OS — PCI Class, Subclass & Programming Interface Definitions
//!
//! Provides descriptive categorization for PCI devices based on the PCI SIG specification.

/// Returns the human-readable name of the PCI Base Class.
pub fn get_class_name(class: u8) -> &'static str {
    match class {
        0x00 => "Unclassified Device",
        0x01 => "Mass Storage Controller",
        0x02 => "Network Controller",
        0x03 => "Display Controller",
        0x04 => "Multimedia Controller",
        0x05 => "Memory Controller",
        0x06 => "Bridge Device",
        0x07 => "Simple Communication Controller",
        0x08 => "Base System Peripheral",
        0x09 => "Input Device Controller",
        0x0A => "Docking Station",
        0x0B => "Processor",
        0x0C => "Serial Bus Controller",
        0x0D => "Wireless Controller",
        0x0E => "Intelligent I/O Controller",
        0x0F => "Satellite Communication Controller",
        0x10 => "Encryption/Decryption Controller",
        0x11 => "Data Acquisition & Signal Processing",
        _ => "Unknown Class",
    }
}

/// Returns the human-readable name of the PCI Subclass.
pub fn get_subclass_name(class: u8, subclass: u8) -> &'static str {
    match (class, subclass) {
        // Mass Storage
        (0x01, 0x00) => "SCSI Bus Controller",
        (0x01, 0x01) => "IDE Interface",
        (0x01, 0x02) => "Floppy Disk Controller",
        (0x01, 0x03) => "IPI Bus Controller",
        (0x01, 0x04) => "RAID Bus Controller",
        (0x01, 0x05) => "ATA Controller",
        (0x01, 0x06) => "Serial ATA (SATA / AHCI)",
        (0x01, 0x07) => "Serial Attached SCSI (SAS)",
        (0x01, 0x08) => "Non-Volatile Memory (NVMe)",
        (0x01, 0x80) => "Other Mass Storage",

        // Network
        (0x02, 0x00) => "Ethernet Controller",
        (0x02, 0x01) => "Token Ring Controller",
        (0x02, 0x02) => "FDDI Controller",
        (0x02, 0x03) => "ATM Controller",
        (0x02, 0x04) => "ISDN Controller",
        (0x02, 0x80) => "Other Network Controller",

        // Display
        (0x03, 0x00) => "VGA-Compatible Display Controller",
        (0x03, 0x01) => "XGA Controller",
        (0x03, 0x02) => "3D Controller",
        (0x03, 0x80) => "Other Display Controller",

        // Multimedia
        (0x04, 0x00) => "Video Device",
        (0x04, 0x01) => "Audio Device (AC'97)",
        (0x04, 0x02) => "Telephony Device",
        (0x04, 0x03) => "High Definition Audio (HDA)",
        (0x04, 0x80) => "Other Multimedia Device",

        // Bridge
        (0x06, 0x00) => "Host Bridge",
        (0x06, 0x01) => "ISA Bridge",
        (0x06, 0x02) => "EISA Bridge",
        (0x06, 0x03) => "MicroChannel Bridge",
        (0x06, 0x04) => "PCI-to-PCI Bridge",
        (0x06, 0x05) => "PCMCIA Bridge",
        (0x06, 0x06) => "NuBus Bridge",
        (0x06, 0x07) => "CardBus Bridge",
        (0x06, 0x80) => "Other Bridge",

        // Serial Bus
        (0x0C, 0x00) => "FireWire (IEEE 1394)",
        (0x0C, 0x01) => "ACCESS Bus",
        (0x0C, 0x02) => "SSA Controller",
        (0x0C, 0x03) => "USB Controller",
        (0x0C, 0x04) => "Fibre Channel",
        (0x0C, 0x05) => "SMBus Controller",
        (0x0C, 0x80) => "Other Serial Bus",

        _ => "General Subclass",
    }
}

/// Returns the specific programming interface description (e.g. UHCI, EHCI, xHCI).
pub fn get_prog_if_name(class: u8, subclass: u8, prog_if: u8) -> &'static str {
    match (class, subclass, prog_if) {
        (0x0C, 0x03, 0x00) => "USB UHCI",
        (0x0C, 0x03, 0x10) => "USB OHCI",
        (0x0C, 0x03, 0x20) => "USB EHCI (USB 2.0)",
        (0x0C, 0x03, 0x30) => "USB xHCI (USB 3.0)",
        (0x01, 0x06, 0x01) => "AHCI 1.0",
        (0x01, 0x08, 0x02) => "NVM Express (NVMe)",
        _ => "Standard Interface",
    }
}
