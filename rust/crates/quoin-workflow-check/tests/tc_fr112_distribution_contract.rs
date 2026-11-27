// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Release workflows preserve Quoin's npm distribution contract.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "workflow contract assertions panic to report the violated release obligation"
)]

use ix_trace_rs::trace;
use serde_yaml_ng::Value;
use std::fs;
use std::path::PathBuf;

fn workflow(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.github/workflows")
        .join(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workflow {}: {error}", path.display()));
    serde_yaml_ng::from_str(&source)
        .unwrap_or_else(|error| panic!("parse workflow {}: {error}", path.display()))
}

fn steps<'a>(workflow: &'a Value, job: &str) -> &'a [Value] {
    workflow["jobs"][job]["steps"]
        .as_sequence()
        .expect("workflow job has steps")
}

fn step<'a>(workflow: &'a Value, job: &str, name: &str) -> &'a Value {
    steps(workflow, job)
        .iter()
        .find(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("workflow job {job} has step {name}"))
}

fn run(step: &Value) -> &str {
    step["run"]
        .as_str()
        .expect("workflow step has a shell script")
}

#[trace("FR-112-AC-1")]
#[test]
fn tc_fr112_npm_release_smokes_the_version_on_every_supported_platform() {
    let release = workflow("release.yml");
    let job = &release["jobs"]["smoke-npm"];
    assert_eq!(job["needs"].as_str(), Some("publish-npm"));
    assert_eq!(job["if"].as_str(), Some("inputs.publish"));

    let platforms = job["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("npm smoke matrix lists each supported platform");
    let observed: Vec<_> = platforms
        .iter()
        .map(|platform| {
            (
                platform["target"].as_str().expect("platform target"),
                platform["native-package"]
                    .as_str()
                    .expect("platform package"),
            )
        })
        .collect();
    assert_eq!(
        observed,
        [
            ("linux-x64", "quoin-linux-x64"),
            ("linux-arm64", "quoin-linux-arm64"),
            ("darwin-arm64", "quoin-darwin-arm64"),
            ("win32-x64", "quoin-win32-x64"),
        ]
    );

    let unix = run(step(
        &release,
        "smoke-npm",
        "Install and verify the published package on Unix",
    ));
    assert!(unix.contains("npm install --global --prefix"), "{unix}");
    assert!(unix.contains("@agent-ix/quoin@${VERSION}"), "{unix}");
    assert!(unix.contains("\"$PREFIX/bin/quoin\" --version"), "{unix}");
    assert!(
        unix.contains("[ \"$actual_version\" != \"$VERSION\" ]"),
        "{unix}"
    );
    assert!(
        unix.contains("quoin-workflow-check smoke")
            && unix.contains("@agent-ix/ix-flow/bin/ix-flow.js"),
        "linux-x64 install keeps the packaged workflow integration smoke: {unix}"
    );

    let windows = run(step(
        &release,
        "smoke-npm",
        "Install and verify the published package on Windows",
    ));
    assert!(
        windows.contains("npm install --global --prefix"),
        "{windows}"
    );
    assert!(
        windows.contains("\"@agent-ix/quoin@$version\""),
        "{windows}"
    );
    assert!(
        windows.contains("Join-Path $prefix 'quoin.cmd'"),
        "{windows}"
    );
    assert!(windows.contains("$actualVersion -ne $version"), "{windows}");
}

#[trace("FR-112-AC-2")]
#[test]
fn tc_fr112_npm_repack_requires_the_selected_release_manifest_assets() {
    let release = workflow("release.yml");
    assert_eq!(
        step(&release, "publish-npm", "Download the release")["env"]["TAG"].as_str(),
        Some("${{ inputs.tag }}")
    );
    assert!(
        run(step(&release, "publish-npm", "Download the release"))
            .contains("gh release download \"$TAG\" --repo \"$GITHUB_REPOSITORY\" -D release")
    );

    let version = run(step(&release, "publish-npm", "Compute the npm version"));
    assert!(version.contains("VERSION=${TAG#v}"), "{version}");

    let manifest = run(step(&release, "publish-npm", "Verify the release manifest"));
    for required in [
        ".schema_version == 2",
        ".version == $v",
        "(.artifacts | length == 4)",
    ] {
        assert!(
            manifest.contains(required),
            "manifest check omits {required}: {manifest}"
        );
    }

    let extract = run(step(&release, "publish-npm", "Extract release assets"));
    assert!(extract.contains("jq -c '.artifacts[]' release/quoin-update-manifest.json"));
    assert!(extract.contains("file=\"release/$asset\""));
    assert!(extract.contains("test -f \"$file\""));
    assert!(extract.contains("case \"$archive\" in"));

    let packaging = step(
        &release,
        "publish-npm",
        "Package and publish the npm distribution",
    );
    assert_eq!(
        packaging["uses"].as_str(),
        Some("agent-ix/nodejs-actions/publish-native-npm@main")
    );
    assert_eq!(
        packaging["with"]["artifacts-dir"].as_str(),
        Some("artifacts")
    );

    for build in steps(&release, "publish-npm") {
        let Some(script) = build["run"].as_str() else {
            continue;
        };
        assert!(
            !script.contains("cargo build --release -p quoin-cli"),
            "{script}"
        );
        assert!(
            !script.contains("cargo build --release --bin quoin"),
            "{script}"
        );
    }
}

#[trace("FR-112-AC-4")]
#[test]
fn tc_fr112_smoke_checks_glibc_metadata_for_both_linux_platform_packages() {
    let release = workflow("release.yml");
    let platforms = release["jobs"]["smoke-npm"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("npm smoke matrix lists platform packages");
    let linux_packages: Vec<_> = platforms
        .iter()
        .filter(|platform| platform["linux"].as_bool() == Some(true))
        .filter_map(|platform| platform["native-package"].as_str())
        .collect();
    assert_eq!(linux_packages, ["quoin-linux-x64", "quoin-linux-arm64"]);

    let script = run(step(
        &release,
        "smoke-npm",
        "Install and verify the published package on Unix",
    ));
    assert!(
        script.contains("if [ \"$LINUX_PACKAGE\" = true ]; then"),
        "{script}"
    );
    assert!(
        script.contains("$PREFIX/lib/node_modules/@agent-ix/$NATIVE_PACKAGE/package.json"),
        "{script}"
    );
    assert!(
        script.contains("JSON.stringify(pkg.libc) !== JSON.stringify([\"glibc\"])"),
        "{script}"
    );
}

#[trace("FR-112-AC-6")]
#[test]
fn tc_fr112_selftest_gates_linux_assets_and_marks_other_targets_not_run() {
    let native = workflow("native-release-build.yml");
    let package_steps = steps(&native, "package");
    let selftest_index = package_steps
        .iter()
        .position(|step| step["name"].as_str() == Some("Run the Linux self-test or record not run"))
        .expect("native package job runs or records the self-test");
    let upload_index = package_steps
        .iter()
        .position(|step| step["uses"].as_str() == Some("actions/upload-artifact@v7"))
        .expect("native package job uploads the artifact");
    assert!(
        selftest_index < upload_index,
        "self-test runs before artifact upload"
    );

    let script = run(&package_steps[selftest_index]);
    assert!(script.contains("set -euo pipefail"), "{script}");
    assert!(
        script.contains("x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu"),
        "{script}"
    );
    let invocation = script
        .lines()
        .find(|line| line.contains("selftest --run --out"))
        .expect("Linux self-test invocation exists");
    assert_eq!(
        invocation.trim(),
        "\"$BINARY\" selftest --run --out \"$RUNNER_TEMP/quoin-selftest-$TARGET.json\""
    );
    assert!(
        !invocation.contains("||"),
        "Linux self-test failure must propagate through the shell: {invocation}"
    );
    assert!(script.contains("not run on %s"), "{script}");
    assert!(
        script.contains("tee -a \"$GITHUB_STEP_SUMMARY\""),
        "{script}"
    );
}
