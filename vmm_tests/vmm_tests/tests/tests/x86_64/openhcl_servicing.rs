// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Integration tests for x86_64 OpenHCL servicing.

use disk_backend_resources::LayeredDiskHandle;
use disk_backend_resources::layer::RamDiskLayerHandle;
use guid::Guid;
use hvlite_defs::config::DeviceVtl;
use petri::OpenHclServicingFlags;
use petri::ResolvedArtifact;
use petri::openvmm::PetriVmConfigOpenVmm;
use petri::pipette::cmd;
use petri_artifacts_vmm_test::artifacts::openhcl_igvm::LATEST_LINUX_DIRECT_TEST_X64;
use scsidisk_resources::SimpleScsiDiskHandle;
use storvsp_resources::ScsiControllerHandle;
use storvsp_resources::ScsiDeviceAndPath;
use storvsp_resources::ScsiPath;
use vm_resource::IntoResource;
use vmm_core_defs::HaltReason;
use vmm_test_macros::openvmm_test;
use std::fs::File;
use disk_backend_resources::layer::DiskLayerHandle;
use disk_backend_resources::FileDiskHandle;
use hvlite_defs::config::VpciDeviceConfig;
use nvme_resources::NvmeControllerHandle;
use nvme_resources::NamespaceDefinition;


fn new_test_vtl2_nvme_device(
    nsid: u32,
    size: u64,
    instance_id: Guid,
    backing_file: Option<File>,
) -> VpciDeviceConfig {
    let layer = if let Some(file) = backing_file {
        LayeredDiskHandle::single_layer(DiskLayerHandle(FileDiskHandle(file).into_resource()))
    } else {
        LayeredDiskHandle::single_layer(RamDiskLayerHandle { len: Some(size) })
    };

    VpciDeviceConfig {
        vtl: DeviceVtl::Vtl2,
        instance_id,
        resource: NvmeControllerHandle {
            subsystem_id: instance_id,
            max_io_queues: 64,
            msix_count: 64,
            namespaces: vec![NamespaceDefinition {
                nsid,
                disk: layer.into_resource(),
                read_only: false,
            }],
        }
        .into_resource(),
    }
}

async fn openhcl_servicing_core(
    config: PetriVmConfigOpenVmm,
    openhcl_cmdline: &str,
    new_openhcl: ResolvedArtifact<impl petri_artifacts_common::tags::IsOpenhclIgvm>,
    flags: OpenHclServicingFlags,
) -> anyhow::Result<()> {
    let (mut vm, agent) = config
        .with_openhcl_command_line(openhcl_cmdline)
        .run()
        .await?;

    agent.ping().await?;

    // Test that inspect serialization works with the old version.
    vm.test_inspect_openhcl().await?;

    vm.restart_openhcl(new_openhcl, flags).await?;

    agent.ping().await?;

    // Test that inspect serialization works with the new version.
    vm.test_inspect_openhcl().await?;

    agent.power_off().await?;
    assert_eq!(vm.wait_for_teardown().await?, HaltReason::PowerOff);

    Ok(())
}

/// Test servicing an OpenHCL VM from the current version to itself.
#[openvmm_test(openhcl_linux_direct_x64 [LATEST_LINUX_DIRECT_TEST_X64])]
async fn openhcl_servicing(
    config: PetriVmConfigOpenVmm,
    (igvm_file,): (ResolvedArtifact<impl petri_artifacts_common::tags::IsOpenhclIgvm>,),
) -> Result<(), anyhow::Error> {
    openhcl_servicing_core(config, "", igvm_file, OpenHclServicingFlags::default()).await
}

/// Test servicing an OpenHCL VM from the current version to itself
/// with VF keepalive support.
#[openvmm_test(openhcl_linux_direct_x64 [LATEST_LINUX_DIRECT_TEST_X64])]
async fn openhcl_servicing_keepalive(
    config: PetriVmConfigOpenVmm,
    (igvm_file,): (ResolvedArtifact<impl petri_artifacts_common::tags::IsOpenhclIgvm>,),
) -> Result<(), anyhow::Error> {
    openhcl_servicing_core(
        config,
        "OPENHCL_ENABLE_VTL2_GPA_POOL=512",
        igvm_file,
        OpenHclServicingFlags {
            enable_nvme_keepalive: true,
        },
    )
    .await
}

/// Test servicing an OpenHCL VM from the current version to itself
/// with VF keepalive support while inducing a storage load on the attached drive
#[openvmm_test(openhcl_uefi_x64(vhd(ubuntu_2204_server_x64)))]
async fn openhcl_servicing_keepalive_storage_load(
    config: PetriVmConfigOpenVmm
) -> Result<(), anyhow::Error> {
    let flags = OpenHclServicingFlags {
        enable_nvme_keepalive: true,
    };

    const NVME_INSTANCE: Guid = guid::guid!("dce4ebad-182f-46c0-8d30-8446c1c62ab3");
    let vtl2_lun = 5;
    let vtl0_scsi_lun = 0;
    let vtl0_nvme_lun = 1;
    let vtl2_nsid = 37;
    let scsi_instance = Guid::new_random();
    let scsi_disk_sectors = 0x2000;
    let nvme_disk_sectors: u64 = 0x300000;
    let sector_size = 512;

    let (vm, agent) = config
        .with_vmbus_redirect()
        .with_custom_config(|c| {
            c.vpci_devices.push(new_test_vtl2_nvme_device(
                vtl2_nsid,
                nvme_disk_sectors * sector_size,
                NVME_INSTANCE,
                None,
            ));
        })
        .with_custom_vtl2_settings(|v| {
            v.dynamic.as_mut().unwrap().storage_controllers.push(
                vtl2_settings_proto::StorageController {
                    instance_id: scsi_instance.to_string(),
                    protocol: vtl2_settings_proto::storage_controller::StorageProtocol::Scsi.into(),
                    luns: vec![
                        vtl2_settings_proto::Lun {
                            location: vtl0_nvme_lun,
                            device_id: Guid::new_random().to_string(),
                            vendor_id: "OpenVMM".to_string(),
                            product_id: "Disk".to_string(),
                            product_revision_level: "1.0".to_string(),
                            serial_number: "0".to_string(),
                            model_number: "1".to_string(),
                            physical_devices: Some(vtl2_settings_proto::PhysicalDevices {
                                r#type: vtl2_settings_proto::physical_devices::BackingType::Single
                                    .into(),
                                device: Some(vtl2_settings_proto::PhysicalDevice {
                                    device_type:
                                        vtl2_settings_proto::physical_device::DeviceType::Nvme
                                            .into(),
                                    device_path: NVME_INSTANCE.to_string(),
                                    sub_device_path: vtl2_nsid,
                                }),
                                devices: Vec::new(),
                            }),
                            ..Default::default()
                        },
                    ],
                    io_queue_depth: None,
                },
            )
        })
        .run()
        .await?;
    


    agent.ping().await?;
    let sh = agent.unix_shell();

    const TEST_FILE: &str = "test_script.sh";
    const TEST_CONTENT: &str = include_str!("../../../test_data/test_script.sh");
    let drive_size = nvme_disk_sectors * sector_size;
    let TEST_CONTENT_WITH_INPUT = str::replace(TEST_CONTENT, "$input", &drive_size.to_string());
    
    agent.write_file(TEST_FILE, TEST_CONTENT_WITH_INPUT.as_bytes()).await?;
    assert_eq!(agent.read_file(TEST_FILE).await?, TEST_CONTENT_WITH_INPUT.as_bytes());

    let sh = agent.unix_shell();
    cmd!(sh, "chmod +x test_script.sh").read().await?;
    cmd!(sh, "./test_script.sh").read().await?;
    // cmd!(sh, "df -h").run().await?;
    // cmd!(sh, "mkdir -p /mnt/nvme_disk").run().await?;


    // cmd!(sh, "echo $nvme_drive_size").run().await?;
    // cmd!(sh, "mkfs.ext4 ${nvme_drive}1").run().await?;
    // cmd!(sh, "mount -t ext4 ${nvme_drive}1 /mnt/nvme_disk").run().await?;
    // cmd!(sh, "fdisk -l").run().await?;
    // cmd!(sh, "time dd if=/dev/zero of=file.txt bs=1M count=500").run().await?;
    // cmd!(sh, "df -h").run().await?;

    // Make sure the disk showed up.
    // cmd!(sh, "apt --version").run().await?;
    // cmd!(sh, "dpkg -l").run().await?;
    // cmd!(sh, "echo hello | ").run().await?;
    // cmd!(sh, "busybox").run().await?;
    // cmd!(sh, "ls /").run().await?;
    // cmd!(sh, "ls /dev/sda").run().await?;
    // cmd!(sh, "ls etc/").run().await?;
    // cmd!(sh, "ls mnt/").run().await?;
    // cmd!(sh, "find /etc/").run().await?;
    // cmd!(sh, "stat -c %s file.txt").run().await?;
    // cmd!(sh, "touch script.sh").run().await?;
    // cmd!(sh, "sed -e -i '$aecho HelloWorld!' script.sh").run().await?;
    // cmd!(sh, "chmod +x script.sh").run().await?;
    // cmd!(sh, "script script.sh").run().await?;
    // cmd!(sh, "fdisk /dev/sda").run().await?;
    // cmd!(sh, "fdisk -l").run().await?;
    // cmd!(sh, "df -h").run().await?;
    // cmd!(sh, "mount /dev/sdb /mnt/disk1").run().await?;
    // cmd!(sh, "df -h").run().await?;
    // cmd!(sh, "cat /etc/hosts").run().await?;
    // cmd!(sh, "lsblk -d -o NAME,SIZE,MODEL").run().await?;
    // println!("{:?}", output);

    // // Test that inspect serialization works with the old version.
    // vm.test_inspect_openhcl().await?;

    // vm.restart_openhcl(igvm_file, flags).await?;

    // agent.ping().await?;
        
    // // Test that inspect serialization works with the new version.
    // vm.test_inspect_openhcl().await?;

    agent.power_off().await?;
    assert_eq!(vm.wait_for_teardown().await?, HaltReason::PowerOff);

    Ok(())
}

#[openvmm_test(openhcl_linux_direct_x64 [LATEST_LINUX_DIRECT_TEST_X64])]
async fn openhcl_servicing_shutdown_ic(
    config: PetriVmConfigOpenVmm,
    (igvm_file,): (ResolvedArtifact<impl petri_artifacts_common::tags::IsOpenhclIgvm>,),
) -> Result<(), anyhow::Error> {
    let (mut vm, agent) = config
        .with_vmbus_redirect()
        .with_custom_config(|c| {
            // Add a disk so that we can make sure (non-intercepted) relay
            // channels are also functional.
            c.vmbus_devices.push((
                DeviceVtl::Vtl0,
                ScsiControllerHandle {
                    instance_id: guid::Guid::new_random(),
                    max_sub_channel_count: 1,
                    devices: vec![ScsiDeviceAndPath {
                        path: ScsiPath {
                            path: 0,
                            target: 0,
                            lun: 0,
                        },
                        device: SimpleScsiDiskHandle {
                            disk: LayeredDiskHandle::single_layer(RamDiskLayerHandle {
                                len: Some(256 * 1024),
                            })
                            .into_resource(),
                            read_only: false,
                            parameters: Default::default(),
                        }
                        .into_resource(),
                    }],
                    io_queue_depth: None,
                    requests: None,
                }
                .into_resource(),
            ));
        })
        .run()
        .await?;
    agent.ping().await?;
    let sh = agent.unix_shell();

    // Make sure the disk showed up.
    cmd!(sh, "ls /dev/sda").run().await?;

    let shutdown_ic = vm.wait_for_enlightened_shutdown_ready().await?;
    vm.restart_openhcl(igvm_file, OpenHclServicingFlags::default())
        .await?;
    // VTL2 will disconnect and then reconnect the shutdown IC across a servicing event.
    tracing::info!("waiting for shutdown IC to close");
    shutdown_ic.await.unwrap_err();
    vm.wait_for_enlightened_shutdown_ready().await?;

    // Make sure the VTL0 disk is still present by reading it.
    agent.read_file("/dev/sda").await?;

    vm.send_enlightened_shutdown(petri::ShutdownKind::Shutdown)
        .await?;
    assert_eq!(vm.wait_for_teardown().await?, HaltReason::PowerOff);
    Ok(())
}

// TODO: add tests with guest workloads while doing servicing.
// TODO: add tests from previous release branch to current.
