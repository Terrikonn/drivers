#![cfg_attr(not(test), no_std)]
#![feature(core_intrinsics)]
//! Register information and driver to program xAPIC, X2APIC and I/O APIC

use bit_field::BitField;

pub mod ioapic;
pub mod x2apic;
pub mod xapic;

/// Specify IPI Delivery mode
#[derive(Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum DeliveryMode {
    /// Delivers the interrupt specified in the vector field to the target
    /// processor or processors.
    Fixed = 0,
    /// Same as fixed mode, except that the interrupt is delivered to the
    /// processor executing at the lowest priority among the set of
    /// processors specified in the destination field. The ability
    /// for a processor to send a lowest priority IPI is model specific and
    /// shoudl be avoided by BIOS and operating system software.
    LowestPriority,
    /// Delivers an SMI interrupt to the target processor or processors.
    /// The vector field must be programmed to 00H for future compatibility.
    Smi,
    /// Reserved
    _Reserved,
    /// Delivers an NMI interrupt to the target processor or processors.
    /// The vector information is ignored.
    Nmi,
    /// Delivers an INIT request to the target processor or processors, which
    /// causes them to perform an INIT.
    Init,
    /// Sends a special start-up IPI (called a SIPI) to the target processor or
    /// processors. The vector typically points to a start-up routine that
    /// is part of the BIOS boot-strap code (see Section 8.4,
    /// Multiple-Processor (MP) Initialization). IPIs sent with this
    /// delivery mode are not automatically retried if the source
    /// APIC is unsable to deliver it. It is up to the software to deter-mine if
    /// the SIPI was not successfully delivered and to reissue the SIPI if
    /// necessary.
    StartUp,
}

/// Specify IPI Destination Mode.
#[derive(Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum DestinationMode {
    Physical = 0,
    Logical,
}

/// Specify Delivery Status
#[derive(Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum DeliveryStatus {
    Idle = 0,
    SendPending,
}

/// IPI Level
#[derive(Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Level {
    Deassert = 0,
    Assert,
}

/// IPI Trigger Mode
#[derive(Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum TriggerMode {
    Edge = 0,
    Level,
}

/// IPI Destination Shorthand
#[derive(Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum DestinationShorthand {
    NoShorthand = 0,
    Myself,
    AllIncludingSelf,
    AllExcludingSelf,
}

// TODO: reimplement with bitte as packed struct
/// Abstaract the IPI control register
#[derive(Debug, PartialEq, Eq)]
pub struct Icr(u64);

impl Icr {
    fn id_to_xapic_destination(destination: ApicId) -> u64 {
        // XApic destination are encoded in bytes 56--63 in the Icr
        match destination {
            ApicId::XApic(d) => (d as u64) << 56,
            ApicId::X2Apic(_d) => {
                unreachable!("x2APIC IDs are not supported for xAPIC (use the x2APIC controller)")
            },
        }
    }

    fn id_to_x2apic_destination(destination: ApicId) -> u64 {
        // whereas, X2Apic destinations are encoded in bytes 32--63 in the Icr
        // The ACPI tables will can name the first 255 processors
        // with xAPIC IDs and no x2APIC entry exists in SRAT
        // However, the IDs should be compatible (I hope)
        let d: u64 = match destination {
            ApicId::XApic(d) => d as u64,
            ApicId::X2Apic(d) => d as u64,
        };

        d << 32
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        dest_encoder: fn(ApicId) -> u64,
        vector: u8,
        destination: ApicId,
        destination_shorthand: DestinationShorthand,
        delivery_mode: DeliveryMode,
        destination_mode: DestinationMode,
        delivery_status: DeliveryStatus,
        level: Level,
        trigger_mode: TriggerMode,
    ) -> Icr {
        Icr(dest_encoder(destination) |
            (destination_shorthand as u64) << 18 |
            (trigger_mode as u64) << 15 |
            (level as u64) << 14 |
            (delivery_status as u64) << 12 |
            (destination_mode as u64) << 11 |
            (delivery_mode as u64) << 8 |
            (vector as u64))
    }

    /// Short-hand to create a Icr value that will work for an x2APIC
    /// controller.
    #[allow(clippy::too_many_arguments)]
    pub fn for_x2apic(
        vector: u8,
        destination: ApicId,
        destination_shorthand: DestinationShorthand,
        delivery_mode: DeliveryMode,
        destination_mode: DestinationMode,
        delivery_status: DeliveryStatus,
        level: Level,
        trigger_mode: TriggerMode,
    ) -> Icr {
        Icr::new(
            Icr::id_to_x2apic_destination,
            vector,
            destination,
            destination_shorthand,
            delivery_mode,
            destination_mode,
            delivery_status,
            level,
            trigger_mode,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn for_xapic(
        vector: u8,
        destination: ApicId,
        destination_shorthand: DestinationShorthand,
        delivery_mode: DeliveryMode,
        destination_mode: DestinationMode,
        delivery_status: DeliveryStatus,
        level: Level,
        trigger_mode: TriggerMode,
    ) -> Icr {
        Icr::new(
            Icr::id_to_xapic_destination,
            vector,
            destination,
            destination_shorthand,
            delivery_mode,
            destination_mode,
            delivery_status,
            level,
            trigger_mode,
        )
    }

    /// Get lower 32-bits of the Icr register.
    pub fn lower(&self) -> u32 {
        self.0 as u32
    }

    /// Get upper 32-bits of the Icr register.
    pub fn upper(&self) -> u32 {
        (self.0 >> 32) as u32
    }
}

/// Encodes the id of a core.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ApicId {
    /// A core destination encoded as an xAPIC ID.
    XApic(u8),
    /// A core destination encoded as an x2APIC ID.
    X2Apic(u32),
}

impl ApicId {
    /// Returns the logical x2APIC ID.
    ///
    /// In x2APIC mode, the 32-bit logical x2APIC ID, which can be read from
    /// LDR, is derived from the 32-bit local x2APIC ID:
    /// Logical x2APIC ID = [(x2APIC ID[19:4] << 16) | (1 << x2APIC id[3:0])]
    pub fn x2apic_logical_id(&self) -> u32 {
        self.x2apic_logical_cluster_id() << 16 | 1 << self.x2apic_logical_cluster_address()
    }

    /// Returns the logical address relative to a cluster
    /// for a givern APIC ID (assuming x2APIC addressing).
    pub fn x2apic_logical_cluster_address(&self) -> u32 {
        self.as_u32().get_bits(0..4)
    }

    /// Returns the cluster ID a given APIC ID belongs to
    /// (assuming x2APIC addressing).
    pub fn x2apic_logical_cluster_id(&self) -> u32 {
        self.as_u32().get_bits(4..20)
    }

    fn as_u32(&self) -> u32 {
        match *self {
            // We support conversion for XApic IDs too because ACPI can
            // report < 255 cores as XApic entries
            ApicId::XApic(id) => u32::from(id),
            ApicId::X2Apic(id) => id,
        }
    }
}

impl From<ApicId> for usize {
    fn from(apic_id: ApicId) -> Self {
        match apic_id {
            ApicId::XApic(id) => usize::from(id),
            ApicId::X2Apic(id) => id as usize,
        }
    }
}

/// Abstracts common interface of local APIC (x2APIC, xAPIC) hardware devices.
pub trait ApicControl {
    /// Is a bootstrap processor?
    fn is_bsp(&self) -> bool;

    /// Return APIC ID.
    fn id(&self) -> u32;

    /// Returns the logical APIC ID.
    fn logical_id(&self) -> u32;

    /// Read APIC version
    fn version(&self) -> u32;

    /// End Of Interrupt -- Acknowledge interrupt delivery.
    fn eoi(&mut self);

    /// Enable TSC deadline timer.
    fn tsc_enable(&mut self, vector: u8);

    /// Set TSC deadline value.
    fn tsc_set(&self, value: u64);

    /// Send a INIT IPI to a core.
    ///
    /// # Safety
    /// Should only be used to reset or boot a new core.
    unsafe fn ipi_init(&mut self, core: ApicId);

    /// Deassert INIT IPI.
    ///
    /// # Safety
    /// Should only be used to reset or boot a new core.
    unsafe fn ipi_init_deassert(&mut self);

    /// Send a STARTUP IPI to a core.
    ///
    /// # Safety
    /// Should only be used to reset or boot a new core.
    unsafe fn ipi_startup(&mut self, core: ApicId, start_page: u8);

    /// Send a generic IPI.
    ///
    /// # Safety
    /// Interrupts one or multiple cores.
    unsafe fn send_ipi(&mut self, icr: Icr);
}
