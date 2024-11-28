// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use arbitrary::{Arbitrary, Unstructured};
use crate::fuzz_driver::{FuzzDriver, DriverAction};
use crate::fuzz_namespace::{FuzzNamespace, NamespaceAction};
use pal_async::DefaultDriver;

/// Struct that stores variables to fuzz the nvme driver
pub struct FuzzNvmeDriver {
    driver: FuzzDriver,
    namespace: FuzzNamespace,  // TODO: This can be implemented as a queue to test 'create' for
                               // namespaces. Essentially have a list of namespaces we can fuzz.
}

impl FuzzNvmeDriver {
    /// Setup a new fuzz driver that will
    pub async fn new(driver: DefaultDriver) -> Self {
        let (namespace, fuzz_driver) = FuzzDriver::new(driver).await;
        let fuzz_namespace = FuzzNamespace::new(namespace);

        Self {
            driver: fuzz_driver,
            namespace: fuzz_namespace,
        }
    }

    /// Cleans up fuzzing infrastructure properly
    pub async fn shutdown(&self) {
        self.namespace.shutdown().await;
    }

    /// Returns an arbitrary action to be taken. Along with arbitrary values
    pub fn get_arbitrary_action(&self, u: &mut Unstructured<'_>) -> arbitrary::Result<NvmeDriverAction>{
       let action: NvmeDriverAction = u.arbitrary()?; 
       Ok(action)
    }

    /// Executes an action
    pub async fn execute_action(&mut self, action: NvmeDriverAction) {
        match action {
            NvmeDriverAction::NamespaceAction { action } => {
                self.namespace.execute_action(action).await
            }
            NvmeDriverAction::DriverAction { action } => {
                self.driver.execute_action(action).await
            }
        } 
    }
}

#[derive(Debug, Arbitrary)]
pub enum NvmeDriverAction {
    NamespaceAction {
        action: NamespaceAction
    },
    DriverAction {
        action: DriverAction
    }
}