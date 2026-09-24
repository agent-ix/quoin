// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Generic direct-process response adapter for EA campaign execution.
//!
//! A native tool's nonzero exit can be the expected negative control. This
//! adapter records that a bounded direct process response was captured; it
//! leaves domain interpretation to an independent member checker.

use engineering_assurance::producer_execution::{
    ContractBinding, ExitCodeBinding, MalformedResponse, OutputArtifact, ProcessEvidence,
    ProducerResponseAdapter, ResponseBinding, TerminalStatus,
};
use serde::Serialize;
use thiserror::Error;

/// The procedure-facing protocol identifier for captured process evidence.
pub const PROCESS_EVIDENCE_PROTOCOL: &str = "quoin.process-evidence/v1";
/// The implementation kind bound to Quoin's generic direct-process adapter.
pub const PROCESS_EVIDENCE_ADAPTER: &str = "quoin.process-evidence-adapter";
/// The implementation version bound to the current decoder.
pub const PROCESS_EVIDENCE_ADAPTER_VERSION: &str = "1";

/// A selected contract does not identify Quoin's direct-process adapter.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ProcessAdapterError {
    /// The response protocol has an unrecognized kind or version.
    #[error("campaign process-evidence protocol binding is unsupported")]
    Protocol,
    /// The response adapter has an unrecognized kind or version.
    #[error("campaign process-evidence adapter binding is unsupported")]
    Adapter,
}

/// The observation carries no producer-authored pass claim. EA's result owns
/// the complete process evidence, including the terminal code and raw streams.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEvidenceObservation {
    /// This adapter's immutable output contract.
    pub schema_version: &'static str,
}

/// Bound adapter that accepts every normal process exit code for independent
/// domain checking, including expected failures in negative controls.
pub struct ProcessEvidenceAdapter {
    binding: ResponseBinding,
}

impl ProcessEvidenceAdapter {
    /// Bind the exact retained protocol and adapter implementations.
    ///
    /// # Errors
    /// Returns [`ProcessAdapterError`] when either identity is not supported.
    pub fn new(
        protocol: ContractBinding,
        adapter: ContractBinding,
    ) -> Result<Self, ProcessAdapterError> {
        if protocol.kind != PROCESS_EVIDENCE_PROTOCOL || protocol.version != "1" {
            return Err(ProcessAdapterError::Protocol);
        }
        if adapter.kind != PROCESS_EVIDENCE_ADAPTER
            || adapter.version != PROCESS_EVIDENCE_ADAPTER_VERSION
        {
            return Err(ProcessAdapterError::Adapter);
        }
        Ok(Self {
            binding: ResponseBinding {
                protocol,
                adapter,
                exit_codes: ExitCodeBinding::Any,
            },
        })
    }
}

impl ProducerResponseAdapter for ProcessEvidenceAdapter {
    type Observation = ProcessEvidenceObservation;

    fn binding(&self) -> &ResponseBinding {
        &self.binding
    }

    fn decode(
        &self,
        process: &ProcessEvidence,
        _artifacts: &[OutputArtifact],
    ) -> Result<Self::Observation, MalformedResponse> {
        if !matches!(process.terminal_status, Some(TerminalStatus::ExitCode(_)))
            || process.stdout.truncated
            || process.stderr.truncated
        {
            return Err(MalformedResponse);
        }
        Ok(ProcessEvidenceObservation {
            schema_version: PROCESS_EVIDENCE_PROTOCOL,
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "a failed test binding must fail the test"
)]
mod tests {
    use engineering_assurance::producer_execution::{
        CapturedStream, ContentDigest, ProcessEvidence, ProducerResponseAdapter, TerminalStatus,
    };

    use super::{
        PROCESS_EVIDENCE_ADAPTER, PROCESS_EVIDENCE_ADAPTER_VERSION, PROCESS_EVIDENCE_PROTOCOL,
        ProcessEvidenceAdapter,
    };

    fn binding(
        kind: &str,
        version: &str,
    ) -> engineering_assurance::producer_execution::ContractBinding {
        engineering_assurance::producer_execution::ContractBinding {
            kind: kind.to_owned(),
            version: version.to_owned(),
            revision: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            digest: ContentDigest::of_bytes(kind.as_bytes()),
        }
    }

    /// Trace: FR-114-AC-2
    /// Provenance: PLAT-1059
    #[test]
    fn tc_1941_nonzero_native_exit_remains_a_completed_observation() {
        let adapter = ProcessEvidenceAdapter::new(
            binding(PROCESS_EVIDENCE_PROTOCOL, "1"),
            binding(PROCESS_EVIDENCE_ADAPTER, PROCESS_EVIDENCE_ADAPTER_VERSION),
        )
        .expect("supported bindings");
        let stream = CapturedStream {
            bytes: Vec::new(),
            digest: ContentDigest::of_bytes(&[]),
            truncated: false,
        };
        let process = ProcessEvidence {
            terminal_status: Some(TerminalStatus::ExitCode(101)),
            stdout: stream.clone(),
            stderr: stream,
        };
        assert!(adapter.decode(&process, &[]).is_ok());
    }
}
