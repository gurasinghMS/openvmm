// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Protobuf definitions for OpenHCL VTL2 settings documents that are provided
//! by the host via the GET device.

#![expect(missing_docs)]
#![forbid(unsafe_code)]
#![expect(unused_qualifications)] // pbjson-build doesn't use ::fully::qualified::paths.

// These crates are referenced by the generated code. Reference them
// explicitly here so that they are not removed by automated tools (xtask
// unused-deps) that cannot see into the generated code.
use pbjson as _;
use pbjson_types as _;
use prost as _;
use serde as _;

// Generated by [`prost-build`]
include!(concat!(env!("OUT_DIR"), "/underhill.settings.rs"));

// Generated by [`pbjson-build`]
include!(concat!(env!("OUT_DIR"), "/underhill.settings.serde.rs"));
