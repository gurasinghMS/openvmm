// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! A shim layer to fuzz responses from an emulated device.
//! This is the primary fuzzer for the host (a.k.a device) ->
//! openhcl attack surface. Do not sanitize any arbitrary data
//! responses in this routine.
use crate::arbitrary_data;

use chipset_device::mmio::MmioIntercept;
use chipset_device::pci::PciConfigSpace;
use inspect::Inspect;
use inspect::InspectMut;
use pci_core::msi::MsiInterruptSet;
use user_driver::emulated::DeviceSharedMemory;
use user_driver::emulated::EmulatedDevice;
use user_driver::emulated::EmulatedDmaAllocator;
use user_driver::emulated::Mapping;
use user_driver::interrupt::DeviceInterrupt;
use user_driver::DeviceBacking;
use user_driver::DeviceRegisterIo;

/// An EmulatedDevice fuzzer that requires a working EmulatedDevice backend.
#[derive(Inspect)]
pub struct FuzzEmulatedDevice<T: InspectMut> {
    device: EmulatedDevice<T>,
}

/// A Mapping fuzzer that requires a working Mapping back end.
#[derive(Inspect)]
pub struct FuzzMapping<T> {
    mapping: Mapping<T>
}

impl<T: PciConfigSpace + MmioIntercept + InspectMut> FuzzEmulatedDevice<T> {
    /// Creates a new emulated device, wrapping `device`, using the provided MSI controller.
    pub fn new(device: T, msi_set: MsiInterruptSet, shared_mem: DeviceSharedMemory) -> Self {
        Self {
            device: EmulatedDevice::new(device, msi_set, shared_mem),
        }
    }
}

/// Implementation for DeviceBacking trait.
impl<T: 'static + Send + InspectMut + MmioIntercept> DeviceBacking for FuzzEmulatedDevice<T> {
    type Registers = FuzzMapping<T>;
    type DmaAllocator = EmulatedDmaAllocator;

    fn id(&self) -> &str {
        self.device.id()
    }

    fn map_bar(&mut self, n: u8) -> anyhow::Result<Self::Registers> {
        Ok(FuzzMapping {
            mapping: self.device.map_bar(n)?,
        })
    }

    fn host_allocator(&self) -> Self::DmaAllocator {
        self.device.host_allocator()
    }

    /// Arbitrarily decide to passthrough or return arbitrary value.
    fn max_interrupt_count(&self) -> u32 {
        // Case: Fuzz response
        if let Ok(true) = arbitrary_data::<bool>() {
            // Return an abritrary u32
            if let Ok(num) = arbitrary_data::<u32>() {
                return num;
            }
        }

        // Case: Passthrough
        self.device.max_interrupt_count()
    }

    fn map_interrupt(&mut self, msix: u32, _cpu: u32) -> anyhow::Result<DeviceInterrupt> {
        self.device.map_interrupt(msix, _cpu)
    }
}

/// Allow the fuzzer to intercept read/write calls to the underlying Mapping type
impl<T: MmioIntercept + Send> DeviceRegisterIo for FuzzMapping<T> {
    fn read_u32(&self, offset: usize) -> u32 {
        if let Ok(true) = arbitrary_data::<bool>() {
            if let Ok(data) = arbitrary_data::<u32>() {
                println!("Responding (fake) read u32 at offset {} with {}", offset, data);
                return data;
            }
        }

        let val = self.mapping.read_u32(offset);
        println!("Responding (real) read u32 at offset {} with {}", offset, val);
        val
    }

    fn read_u64(&self, offset: usize) -> u64 {
        if let Ok(true) = arbitrary_data::<bool>() {
            if let Ok(data) = arbitrary_data::<u64>() {
                println!("Responding (fake) read u64 at offset {} with {}", offset, data);
                return data;
            }
        }

        let val = self.mapping.read_u64(offset);
        println!("Responding (real) read u64 at offset {} with {}", offset, val);
        val
    }

    fn write_u32(&self, offset: usize, data: u32) {
        println!("Writing u32 offset {} with {}", offset, data);
        self.mapping.write_u32(offset, data)
    }

    fn write_u64(&self, offset: usize, data: u64) {
        println!("Writing u64 offset {} with {}", offset, data);
        self.mapping.write_u64(offset, data)
    }
}
