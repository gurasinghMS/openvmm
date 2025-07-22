use crate::DOORBELL_STRIDE_BITS;
use crate::NvmeController;
use crate::NvmeControllerCaps;
use crate::NvmeControllerClient;
use crate::spec;
use chipset_device::ChipsetDevice;
use chipset_device::io::IoError;
use chipset_device::io::IoError::InvalidRegister;
use chipset_device::io::IoResult;
use chipset_device::mmio::MmioIntercept;
use chipset_device::mmio::RegisterMmioIntercept;
use chipset_device::pci::PciConfigSpace;
use guestmem::GuestMemory;
use inspect::Inspect;
use inspect::InspectMut;
use mesh::Cell;
use pal_async::timer::PolledTimer;
use pci_core::msi::RegisterMsi;
use std::any::Any;
use std::time::Duration;
use vmcore::device_state::ChangeDeviceState;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SaveRestore;
use vmcore::save_restore::SavedStateNotSupported;
use vmcore::vm_task::VmTaskDriver;
use vmcore::vm_task::VmTaskDriverSource;

// The function can respond with two types of actions.
#[derive(Debug, Clone)]
pub enum FaultInjectionAction {
    /// No-fault. Will always run the underlying function with the given input to the function. A direct passthrough
    No_Op,
    /// Drops the request to the underlying function but expects to be given some output for the caller.
    Drop,
    /// Underlying function is called. Given output is passed along to the caller. Output can be different from that of the function that was run.
    Fault,
    // TODO: There are many other types of faults that can be added in the long run. For eg, change output of the underlying function,
    // change input to the underlying function, call the underlying function several times, etc. This is meant to be a flexible model for invoking faults
    // Not every scenario needs to be supported when calling the underlying function for faults.
    // FaultInjectionAction::Delay is a special case that should be implemented by the custom fault injection function. Keep in mind that
    // delay can also be modeled as FaultInjectionAction::Drop for a given duration of time depending on the required outcome.
}

#[derive(InspectMut)]
pub struct NvmeControllerFaultInjection {
    #[inspect(skip)]
    driver: VmTaskDriver,
    #[inspect(skip)]
    inner: NvmeController,
    #[inspect(hex, with = "|x| inspect::AsDebug(x.get())")]
    admin_delay: Cell<Duration>,
}

#[derive(Inspect)]
struct Regs {
    asq: u64,
    acq: u64,
    aqa: spec::Aqa,
    cc: spec::Cc,
}

impl NvmeControllerFaultInjection {
    /// Creates a new NVMe controller with fault injection.
    pub fn new(
        driver_source: &VmTaskDriverSource,
        guest_memory: GuestMemory,
        register_msi: &mut dyn RegisterMsi,
        register_mmio: &mut dyn RegisterMmioIntercept,
        caps: NvmeControllerCaps,
        admin_delay: Cell<Duration>,
    ) -> Self {
        Self {
            driver: driver_source.simple(),
            inner: NvmeController::new(
                driver_source,
                guest_memory.clone(),
                register_msi,
                register_mmio,
                caps,
            ),
            admin_delay,
        }
    }

    /// Returns a client for manipulating the NVMe controller at runtime.
    pub fn client(&self) -> NvmeControllerClient {
        self.inner.client()
    }

    /// Reads from the virtual BAR 0.
    pub fn read_bar0(&mut self, addr: u16, data: &mut [u8]) -> IoResult {
        self.inner.read_bar0(addr, data)
    }

    /// Writes to the virtual BAR 0.
    pub fn write_bar0(&mut self, addr: u16, data: &[u8]) -> IoResult {
        if addr >= 0x1000 {
            // Doorbell write.
            let base = addr - 0x1000;
            let index = base >> DOORBELL_STRIDE_BITS;
            if (index << DOORBELL_STRIDE_BITS) != base {
                return IoResult::Err(InvalidRegister);
            }
            let Ok(data) = data.try_into() else {
                return IoResult::Err(IoError::InvalidAccessSize);
            };
            let _ = u32::from_ne_bytes(data);
            // Delay only the Admin Submission Queue doorbell writes.
            if index == 0 {
                async {
                    PolledTimer::new(&self.driver)
                        .sleep(self.admin_delay.get())
                        .await
                };
            }
        }

        // Handled all queue related jargon, let the inner controller handle the rest
        self.inner.write_bar0(addr, data)
    }

    pub fn fatal_error(&mut self) {
        self.inner.fatal_error();
    }

    fn set_cc(&mut self, cc: spec::Cc) {
        let mask: u32 = u32::from(
            spec::Cc::new()
                .with_en(true)
                .with_shn(0b11)
                .with_iosqes(0b1111)
                .with_iocqes(0b1111),
        );
        let mut cc: spec::Cc = (u32::from(cc) & mask).into();
        if !self.admin.is_running() {}

        self.regs.cc = cc;
    }
}

impl ChangeDeviceState for NvmeControllerFaultInjection {
    fn start(&mut self) {
        self.inner.start();
    }

    async fn stop(&mut self) {
        self.inner.stop().await;
    }

    async fn reset(&mut self) {
        self.inner.reset().await;
    }
}

impl ChipsetDevice for NvmeControllerFaultInjection {
    fn supports_mmio(&mut self) -> Option<&mut dyn MmioIntercept> {
        self.inner.supports_mmio()
    }

    fn supports_pci(&mut self) -> Option<&mut dyn PciConfigSpace> {
        self.inner.supports_pci()
    }
}

impl MmioIntercept for NvmeControllerFaultInjection {
    fn mmio_read(&mut self, addr: u64, data: &mut [u8]) -> IoResult {
        self.inner.mmio_read(addr, data)
    }

    fn mmio_write(&mut self, addr: u64, data: &[u8]) -> IoResult {
        self.inner.mmio_write(addr, data)
    }
}

impl PciConfigSpace for NvmeControllerFaultInjection {
    fn pci_cfg_read(&mut self, offset: u16, value: &mut u32) -> IoResult {
        self.inner.pci_cfg_read(offset, value)
    }

    fn pci_cfg_write(&mut self, offset: u16, value: u32) -> IoResult {
        self.inner.pci_cfg_write(offset, value)
    }
}

impl SaveRestore for NvmeControllerFaultInjection {
    type SavedState = SavedStateNotSupported;

    fn save(&mut self) -> Result<Self::SavedState, SaveError> {
        self.inner.save()
    }

    fn restore(
        &mut self,
        state: Self::SavedState,
    ) -> Result<(), vmcore::save_restore::RestoreError> {
        self.inner.restore(state)
    }
}
