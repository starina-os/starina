//! MP table loader.
//!
//! # MultiProcessor Specification
//!
//! The official document for the MP table. Because the official document is no
//! longer available in intel.com, use a copy of the document from MIT:
//!
//! https://pdos.csail.mit.edu/6.828/2011/readings/ia32/MPspec.pdf
use core::mem::size_of;

use ftl_types::address::PAddr;

use super::io_apic;
use super::local_apic;
use super::paddr2vaddr;

const MP_FLOATPTR_SIGNATURE: [u8; 4] = *b"_MP_";
const MP_CONFIG_TABLE_SIGNATURE: [u8; 4] = *b"PCMP";

/// MP floating pointer structure.
#[derive(Debug)]
#[repr(C, packed)]
pub struct MpFloatPtr {
    /// Signature.
    signature: [u8; 4],
    /// Configuration table physical address.
    config_table_addr: u32,
    /// The length of the floating pointer structure.
    len: u8,
    /// MultiProcessor Specification version.
    version: u8,
    /// Checksum.
    checksum: u8,
    /// Default configuration type.
    default_config_type: u8,
}

/// MP Configuration Table entry: Processor.
#[derive(Debug)]
#[repr(C, packed)]
pub struct MpProcessorEntry {
    /// Entry type (0).
    entry_type: u8,
    /// Local APIC ID.
    local_apic_id: u8,
    /// Local APIC version.
    local_apic_version: u8,
    /// CPU flags.
    cpu_flags: u8,
    /// CPU signature.
    cpu_signature: u32,
    /// Feature flags.
    feature_flags: u32,
    /// Reserved.
    reserved: [u8; 8],
}

/// MP Configuration Table entry: Bus.
#[derive(Debug)]
#[repr(C, packed)]
pub struct MpBusEntry {
    /// Entry type (1).
    entry_type: u8,
    /// Bus ID.
    bus_id: u8,
    /// Bus type string.
    bus_type: [u8; 6],
}

/// MP Configuration Table entry: IO APIC.
#[derive(Debug)]
#[repr(C, packed)]
pub struct MpIoApicEntry {
    /// Entry type (2).
    entry_type: u8,
    /// IO APIC ID.
    io_apic_id: u8,
    /// IO APIC version.
    io_apic_version: u8,
    /// IO APIC flags.
    io_apic_flags: u8,
    /// IO APIC physical address.
    io_apic_addr: u32,
}

/// MP Configuration Table entry: I/O Interrupt Assignment.
#[derive(Debug)]
#[repr(C, packed)]
pub struct MpIoInterruptEntry {
    /// Entry type (3).
    entry_type: u8,
    /// Interrupt type.
    interrupt_type: u8,
    /// Interrupt flags.
    interrupt_flags: u16,
    /// Source bus ID.
    source_bus_id: u8,
    /// Source bus IRQ.
    source_bus_irq: u8,
    /// Destination IO APIC ID.
    dest_io_apic_id: u8,
    /// Destination IO APIC INTIN.
    dest_io_apic_intin: u8,
}

/// MP Configuration Table entry: Local Interrupt Assignment.
#[derive(Debug)]
#[repr(C, packed)]
pub struct MpLocalInterruptEntry {
    /// Entry type (4).
    entry_type: u8,
    /// Interrupt type.
    interrupt_type: u8,
    /// Interrupt flags.
    interrupt_flags: u16,
    /// Source bus ID.
    source_bus_id: u8,
    /// Source bus IRQ.
    source_bus_irq: u8,
    /// Destination local APIC ID.
    dest_local_apic_id: u8,
    /// Destination local APIC LINTIN.
    dest_local_apic_lintin: u8,
}

/// MP Configuration Table header.
#[derive(Debug)]
#[repr(C, packed)]
pub struct MpConfigTable {
    /// Signature.
    signature: [u8; 4],
    /// The length of the configuration table.
    len: u16,
    /// MultiProcessor Specification version.
    version: u8,
    /// Checksum.
    checksum: u8,
    /// OEM ID.
    oem_id: [u8; 8],
    /// Product ID.
    product_id: [u8; 12],
    /// OEM table physical address.
    oem_table_ptr: u32,
    /// The length of the OEM table.
    oem_table_len: u16,
    /// Number of entries in the configuration table.
    entry_count: u16,
    /// Local APIC physical address.
    local_apic_addr: u32,
    /// Extended table length.
    extended_table_len: u16,
    /// Extended table checksum.
    extended_table_checksum: u8,
    /// Reserved.
    reserved: u8,
}

fn find_mp_floatptr_in(paddr: PAddr, len: usize) -> Option<&'static MpFloatPtr> {
    // It scans fhe region in 16-byte increments because the MP spec says:
    //
    // > It must span a minimum of 16 contiguous bytes, beginning on a 16-
    // > boundary
    for offset in (0..len).step_by(16) {
        let floatptr: &'static MpFloatPtr =
            unsafe { &*paddr2vaddr(paddr.add(offset)).unwrap().as_ptr() };
        if floatptr.signature == MP_FLOATPTR_SIGNATURE {
            return Some(floatptr);
        }
    }

    None
}

//// Possible memory locations for the MP floating pointer structure:
///
/// > This structure must be stored in at least one of the following memory
/// > locations, because the operating system searches for the MP floating
/// > pointer structure in the order described below:
/// >
/// > a. In the first kilobyte of Extended BIOS Data Area (EBDA), or
/// >
/// > b. Within the last kilobyte of system base memory (e.g., 639K-640K for
/// >    systems with 640 KB of base memory or 511K-512K for systems with
/// >    512 KB of base memory) if the EBDA segment is undefined, or
/// >
/// > c. In the BIOS ROM address space between 0F0000h and 0FFFFFh
///
/// In our case, we assume it exists in 0xF0000-0xFFFFF for simplicity. If
/// you encounter a real machine that doesn't have the MP floating pointer
/// struct in this region, let me know.
const MPTABLE_RANGES: &[(PAddr, usize)] =
    &[(PAddr::new(0xf0000), 0x10000), (PAddr::new(0x9fc00), 0x400)];

fn find_mp_floatptr() -> Option<&'static MpFloatPtr> {
    let mut floatptr: Option<&'static MpFloatPtr> = None;
    for (paddr, len) in MPTABLE_RANGES {
        if let Some(ptr) = find_mp_floatptr_in(*paddr, *len) {
            return Some(ptr);
        }
    }

    None
}

pub fn init() {
    let floatptr = find_mp_floatptr().expect("MP floating pointer structure not found");

    let config_table: &'static MpConfigTable = unsafe {
        &*paddr2vaddr(PAddr::new(floatptr.config_table_addr as usize))
            .unwrap()
            .as_ptr()
    };

    if config_table.signature != MP_CONFIG_TABLE_SIGNATURE {
        panic!(
            "MP configuration table signature mismatch: {:x?}",
            config_table.signature
        );
    }

    // Print the MP configuration table.
    local_apic::init(PAddr::new(config_table.local_apic_addr as usize));

    // Scan the configuration table entries.
    let mut ioapic_entry: Option<&'static MpIoApicEntry> = None;
    let mut offset = size_of::<MpConfigTable>();
    while offset < config_table.len as usize {
        let mut paddr = PAddr::new(floatptr.config_table_addr as usize + offset);
        let entry_type: u8 = unsafe { *paddr2vaddr(paddr).unwrap().as_ptr() };
        let entry_len = match entry_type {
            0 => size_of::<MpProcessorEntry>(),
            1 => size_of::<MpBusEntry>(),
            2 => {
                ioapic_entry = Some(unsafe {
                    &*(paddr2vaddr(paddr).unwrap().as_ptr() as *const MpIoApicEntry)
                });
                size_of::<MpIoApicEntry>()
            }
            3 => size_of::<MpIoInterruptEntry>(),
            4 => size_of::<MpLocalInterruptEntry>(),
            _ => {
                panic!("unknown MP entry type: {}", entry_type);
            }
        };

        offset += entry_len;
    }

    io_apic::init(PAddr::new(ioapic_entry.unwrap().io_apic_addr as usize));
}
