#!/usr/bin/env python3
"""Regression tests for the deterministic SPDX release SBOM."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from copy import deepcopy


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "release-sbom.py"
SPEC = importlib.util.spec_from_file_location("release_sbom", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load release SBOM module")
sbom = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sbom)

VERSION = "1.2.3-alpha.4"
REVISION = "a" * 40
EPOCH = 1_700_000_000
CORE_ID = "path+file:///repo/core#numinous-core@1.2.3-alpha.4"
APP_ID = "path+file:///repo/app#numinous-app@1.2.3-alpha.4"
SERDE_ID = "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.228"
SERDE_CHECKSUM = "b" * 64


def fixture_metadata() -> dict[str, object]:
    return {
        "packages": [
            {
                "id": APP_ID,
                "license": "Apache-2.0",
                "name": "numinous-app",
                "source": None,
                "version": VERSION,
            },
            {
                "id": CORE_ID,
                "license": "Apache-2.0",
                "name": "numinous-core",
                "source": None,
                "version": VERSION,
            },
            {
                "id": SERDE_ID,
                "license": "MIT/Apache-2.0",
                "name": "serde",
                "source": sbom.CRATES_IO_SOURCE,
                "version": "1.0.228",
            },
        ],
        "resolve": {
            "nodes": [
                {"dependencies": [CORE_ID, SERDE_ID], "id": APP_ID},
                {"dependencies": [SERDE_ID], "id": CORE_ID},
                {"dependencies": [], "id": SERDE_ID},
            ]
        },
        "workspace_members": [CORE_ID, APP_ID],
    }


def fixture_lock() -> bytes:
    return f'''version = 4

[[package]]
name = "numinous-app"
version = "{VERSION}"

[[package]]
name = "numinous-core"
version = "{VERSION}"

[[package]]
name = "serde"
version = "1.0.228"
source = "{sbom.CRATES_IO_SOURCE}"
checksum = "{SERDE_CHECKSUM}"
'''.encode("utf-8")


def fixture_document() -> dict[str, object]:
    return sbom.build_sbom(fixture_metadata(), fixture_lock(), VERSION, REVISION, EPOCH)


def fixture_native_inventory() -> list[dict[str, object]]:
    records = []
    for target, (binary_format, architecture) in sorted(
        sbom.PACKAGE.NATIVE_TARGETS.items()
    ):
        suffix = sbom.PACKAGE.TARGETS[target][0]
        imported = {
            "aarch64-apple-darwin": "/usr/lib/libSystem.B.dylib",
            "x86_64-apple-darwin": "/usr/lib/libSystem.B.dylib",
            "x86_64-pc-windows-msvc": "kernel32.dll",
            "x86_64-unknown-linux-gnu": "libc.so.6",
        }[target]
        for index, binary in enumerate(sbom.PACKAGE.BINARIES):
            records.append(
                {
                    "architecture": architecture,
                    "fileName": f"bin/{binary}{suffix}",
                    "format": binary_format,
                    "imports": [imported],
                    "sha1": f"{index + len(records):040x}"[-40:],
                    "sha256": f"{index + len(records):064x}"[-64:],
                    "sourceRevision": REVISION,
                    "target": target,
                    "version": VERSION,
                }
            )
    return records


class ReleaseSbomTests(unittest.TestCase):
    """Keep the release inventory deterministic, complete, and fail-closed."""

    def test_document_is_deterministic_and_names_the_release(self) -> None:
        first = fixture_document()
        second = fixture_document()
        self.assertEqual(sbom.render_sbom(first), sbom.render_sbom(second))
        self.assertEqual(first["spdxVersion"], "SPDX-2.3")
        self.assertEqual(first["dataLicense"], "CC0-1.0")
        self.assertEqual(first["name"], f"numinous-v{VERSION}-release-sbom")
        self.assertEqual(first["creationInfo"]["created"], "2023-11-14T22:13:20Z")
        self.assertEqual(len(first["packages"]), 4)

    def test_workspace_checkout_path_does_not_change_the_document(self) -> None:
        rebased = deepcopy(fixture_metadata())
        replacements = {
            CORE_ID: CORE_ID.replace("file:///repo/core", "file:///other/core"),
            APP_ID: APP_ID.replace("file:///repo/app", "file:///other/app"),
        }
        for package in rebased["packages"]:
            package["id"] = replacements.get(package["id"], package["id"])
        rebased["workspace_members"] = [
            replacements.get(cargo_id, cargo_id)
            for cargo_id in rebased["workspace_members"]
        ]
        for node in rebased["resolve"]["nodes"]:
            node["id"] = replacements.get(node["id"], node["id"])
            node["dependencies"] = [
                replacements.get(cargo_id, cargo_id)
                for cargo_id in node["dependencies"]
            ]
        self.assertEqual(
            sbom.render_sbom(fixture_document()),
            sbom.render_sbom(
                sbom.build_sbom(rebased, fixture_lock(), VERSION, REVISION, EPOCH)
            ),
        )

    def test_graph_describes_release_workspace_and_dependencies(self) -> None:
        document = fixture_document()
        by_name = {package["name"]: package for package in document["packages"]}
        relationships = {
            (
                item["spdxElementId"],
                item["relationshipType"],
                item["relatedSpdxElement"],
            )
            for item in document["relationships"]
        }
        release_id = by_name["numinous-release"]["SPDXID"]
        self.assertIn(("SPDXRef-DOCUMENT", "DESCRIBES", release_id), relationships)
        self.assertIn(
            (release_id, "CONTAINS", by_name["numinous-app"]["SPDXID"]),
            relationships,
        )
        self.assertIn(
            (
                by_name["numinous-app"]["SPDXID"],
                "DEPENDS_ON",
                by_name["numinous-core"]["SPDXID"],
            ),
            relationships,
        )
        self.assertIn(
            (
                by_name["numinous-core"]["SPDXID"],
                "DEPENDS_ON",
                by_name["serde"]["SPDXID"],
            ),
            relationships,
        )

    def test_registry_package_carries_checksum_download_and_purl(self) -> None:
        document = fixture_document()
        serde = next(item for item in document["packages"] if item["name"] == "serde")
        self.assertEqual(
            serde["checksums"],
            [{"algorithm": "SHA256", "checksumValue": SERDE_CHECKSUM}],
        )
        self.assertEqual(
            serde["downloadLocation"],
            "https://crates.io/api/v1/crates/serde/1.0.228/download",
        )
        self.assertEqual(
            serde["externalRefs"][0]["referenceLocator"],
            "pkg:cargo/serde@1.0.228",
        )
        self.assertEqual(serde["licenseDeclared"], "MIT OR Apache-2.0")

    def test_invalid_license_expression_fails_closed(self) -> None:
        for declaration in (
            "MIT OR",
            "BANANA",
            "+",
            "M+IT",
            "MIT++",
            "DocumentRef-x:Banana",
            "MIT WITH Not-A-Real-Exception",
            "(MIT OR Apache-2.0) WITH LLVM-exception",
        ):
            with self.subTest(declaration=declaration):
                metadata = fixture_metadata()
                metadata["packages"][2]["license"] = declaration
                with self.assertRaisesRegex(sbom.SbomError, "not an SPDX expression"):
                    sbom.build_sbom(metadata, fixture_lock(), VERSION, REVISION, EPOCH)

    def test_namespace_binds_version_revision_and_exact_lockfile(self) -> None:
        document = fixture_document()
        lock_digest = sbom.hashlib.sha256(fixture_lock()).hexdigest()
        native_digest = sbom.hashlib.sha256(b"[]").hexdigest()
        self.assertEqual(
            document["documentNamespace"],
            f"https://github.com/blisspixel/numinous/sbom/{VERSION}/{REVISION}/{lock_digest}/{native_digest}",
        )
        release = next(
            item for item in document["packages"] if item["name"] == "numinous-release"
        )
        self.assertEqual(
            release["externalRefs"][0]["referenceLocator"],
            f"git+https://github.com/blisspixel/numinous.git@{REVISION}",
        )
        changed_lock = fixture_lock().replace(b"version = 4", b"version = 3")
        changed = sbom.build_sbom(
            fixture_metadata(), changed_lock, VERSION, REVISION, EPOCH
        )
        self.assertNotEqual(document["documentNamespace"], changed["documentNamespace"])

    def test_native_inventory_describes_every_binary_and_header_import(self) -> None:
        inventory = fixture_native_inventory()
        document = sbom.build_sbom(
            fixture_metadata(), fixture_lock(), VERSION, REVISION, EPOCH, inventory
        )
        reordered = sbom.build_sbom(
            fixture_metadata(),
            fixture_lock(),
            VERSION,
            REVISION,
            EPOCH,
            list(reversed(inventory)),
        )
        self.assertEqual(sbom.render_sbom(document), sbom.render_sbom(reordered))
        self.assertEqual(len(document["files"]), 12)
        self.assertEqual(len(document["packages"]), 20)
        release = next(
            package
            for package in document["packages"]
            if package["name"] == "numinous-release"
        )
        self.assertIn("exact packaged executable hashes", release["comment"])
        file_ids = {item["SPDXID"] for item in document["files"]}
        native_ids = {
            package["SPDXID"]
            for package in document["packages"]
            if package.get("primaryPackagePurpose") == "LIBRARY"
        }
        artifacts = {
            package["SPDXID"]: package
            for package in document["packages"]
            if package.get("primaryPackagePurpose") == "APPLICATION"
        }
        self.assertEqual(len(artifacts), 12)
        relationships = {
            (
                item["spdxElementId"],
                item["relationshipType"],
                item["relatedSpdxElement"],
            )
            for item in document["relationships"]
        }
        self.assertTrue(
            all(
                (release["SPDXID"], "CONTAINS", artifact_id) in relationships
                for artifact_id in artifacts
            )
        )
        self.assertTrue(
            all(
                any(
                    (artifact_id, "CONTAINS", file_id) in relationships
                    for artifact_id in artifacts
                )
                for file_id in file_ids
            )
        )
        self.assertTrue(all(package["filesAnalyzed"] for package in artifacts.values()))
        self.assertTrue(
            all("packageVerificationCode" in package for package in artifacts.values())
        )
        self.assertTrue(
            all(
                any(
                    (file_id, "DEPENDS_ON", native_id) in relationships
                    for native_id in native_ids
                )
                for file_id in file_ids
            )
        )
        changed_inventory = deepcopy(inventory)
        changed_inventory[0]["sha256"] = "f" * 64
        changed = sbom.build_sbom(
            fixture_metadata(),
            fixture_lock(),
            VERSION,
            REVISION,
            EPOCH,
            changed_inventory,
        )
        self.assertNotEqual(document["documentNamespace"], changed["documentNamespace"])
        changed_sha1 = deepcopy(inventory)
        changed_sha1[0]["sha1"] = "e" * 40
        changed = sbom.build_sbom(
            fixture_metadata(),
            fixture_lock(),
            VERSION,
            REVISION,
            EPOCH,
            changed_sha1,
        )
        self.assertNotEqual(document["documentNamespace"], changed["documentNamespace"])

    def test_native_inventory_identity_and_shape_fail_closed(self) -> None:
        cases: list[tuple[str, list[dict[str, object]], str]] = []
        incomplete = fixture_native_inventory()[:-1]
        cases.append(("incomplete", incomplete, "every release executable"))
        repeated = fixture_native_inventory()
        repeated[-1] = deepcopy(repeated[0])
        cases.append(("repeated", repeated, "repeats"))
        wrong_version = fixture_native_inventory()
        wrong_version[0]["version"] = "9.9.9"
        cases.append(("version", wrong_version, "release identity"))
        wrong_checksum = fixture_native_inventory()
        wrong_checksum[0]["sha256"] = "A" * 64
        cases.append(("checksum", wrong_checksum, "checksum is invalid"))
        wrong_sha1 = fixture_native_inventory()
        wrong_sha1[0]["sha1"] = "A" * 40
        cases.append(("sha1", wrong_sha1, "checksum is invalid"))
        wrong_format = fixture_native_inventory()
        wrong_format[0]["format"] = "PE"
        cases.append(("format", wrong_format, "does not match"))
        repeated_import = fixture_native_inventory()
        repeated_import[0]["imports"] = ["z.so", "z.so"]
        cases.append(("imports", repeated_import, "not unique and sorted"))
        for label, inventory, message in cases:
            with self.subTest(label=label):
                with self.assertRaisesRegex(sbom.SbomError, message):
                    sbom.build_sbom(
                        fixture_metadata(),
                        fixture_lock(),
                        VERSION,
                        REVISION,
                        EPOCH,
                        inventory,
                    )

    def test_missing_lock_entry_and_registry_checksum_fail_closed(self) -> None:
        missing = fixture_lock().split(b'[[package]]\nname = "serde"', maxsplit=1)[0]
        with self.assertRaisesRegex(sbom.SbomError, "no exact entry"):
            sbom.build_sbom(fixture_metadata(), missing, VERSION, REVISION, EPOCH)
        no_checksum = fixture_lock().replace(
            f'checksum = "{SERDE_CHECKSUM}"\n'.encode("utf-8"), b""
        )
        with self.assertRaisesRegex(sbom.SbomError, "no registry checksum"):
            sbom.build_sbom(fixture_metadata(), no_checksum, VERSION, REVISION, EPOCH)
        custom_registry = "registry+https://registry.example/index"
        custom_metadata = fixture_metadata()
        custom_metadata["packages"][2]["source"] = custom_registry
        custom_lock = no_checksum.replace(
            sbom.CRATES_IO_SOURCE.encode("utf-8"), custom_registry.encode("utf-8")
        )
        with self.assertRaisesRegex(sbom.SbomError, "no registry checksum"):
            sbom.build_sbom(custom_metadata, custom_lock, VERSION, REVISION, EPOCH)

    def test_lock_and_resolve_duplicates_fail_closed(self) -> None:
        duplicate_lock = (
            fixture_lock()
            + f'''\n[[package]]
name = "numinous-app"
version = "{VERSION}"
'''.encode("utf-8")
        )
        with self.assertRaisesRegex(sbom.SbomError, "repeats"):
            sbom.build_sbom(
                fixture_metadata(), duplicate_lock, VERSION, REVISION, EPOCH
            )
        duplicate_node = fixture_metadata()
        duplicate_node["resolve"]["nodes"].append({"dependencies": [], "id": SERDE_ID})
        with self.assertRaisesRegex(sbom.SbomError, "does not cover every package"):
            sbom.build_sbom(duplicate_node, fixture_lock(), VERSION, REVISION, EPOCH)

    def test_metadata_identity_and_workspace_duplicates_fail_closed(self) -> None:
        duplicate_package = fixture_metadata()
        repeated = deepcopy(duplicate_package["packages"][2])
        repeated["id"] = "another-cargo-id"
        duplicate_package["packages"].append(repeated)
        with self.assertRaisesRegex(sbom.SbomError, "repeats locked identity"):
            sbom.build_sbom(duplicate_package, fixture_lock(), VERSION, REVISION, EPOCH)
        duplicate_member = fixture_metadata()
        duplicate_member["workspace_members"].append(CORE_ID)
        with self.assertRaisesRegex(sbom.SbomError, "repeats workspace member"):
            sbom.build_sbom(duplicate_member, fixture_lock(), VERSION, REVISION, EPOCH)

    def test_unknown_dependency_and_workspace_version_fail_closed(self) -> None:
        unknown = fixture_metadata()
        unknown["resolve"]["nodes"][0]["dependencies"].append("unknown")
        with self.assertRaisesRegex(sbom.SbomError, "unknown dependency"):
            sbom.build_sbom(unknown, fixture_lock(), VERSION, REVISION, EPOCH)
        mismatch = fixture_metadata()
        mismatch["packages"][0]["version"] = "1.2.4"
        mismatch_lock = fixture_lock().replace(
            f'''name = "numinous-app"
version = "{VERSION}"'''.encode("utf-8"),
            b'''name = "numinous-app"
version = "1.2.4"''',
        )
        with self.assertRaisesRegex(sbom.SbomError, "workspace package versions"):
            sbom.build_sbom(mismatch, mismatch_lock, VERSION, REVISION, EPOCH)

    def test_identity_inputs_are_strict(self) -> None:
        for version, revision, epoch in (
            ("v1.2.3", REVISION, EPOCH),
            (VERSION, "A" * 40, EPOCH),
            (VERSION, REVISION, -1),
        ):
            with self.subTest(version=version, revision=revision, epoch=epoch):
                with self.assertRaises(sbom.SbomError):
                    sbom.build_sbom(
                        fixture_metadata(), fixture_lock(), version, revision, epoch
                    )

    def test_evidence_file_is_exclusive_and_exact(self) -> None:
        expected = sbom.render_sbom(fixture_document())
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "release.spdx.json"
            sbom.write_exclusive(path, expected)
            sbom.verify_file(path, expected)
            with self.assertRaisesRegex(sbom.SbomError, "refusing to replace"):
                sbom.write_exclusive(path, expected)
            path.write_bytes(expected.replace(b"numinous", b"NUMINOUS", 1))
            with self.assertRaisesRegex(sbom.SbomError, "does not match"):
                sbom.verify_file(path, expected)

    def test_file_reads_reject_nonordinary_and_oversize_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(sbom.SbomError, "not an ordinary file"):
                sbom.read_regular_file(root, maximum=1)
            source = root / "source"
            source.write_bytes(b"ab")
            with self.assertRaisesRegex(sbom.SbomError, "exceeds the 1-byte limit"):
                sbom.read_regular_file(source, maximum=1)
            link = root / "link"
            try:
                link.symlink_to(source)
            except OSError:
                return
            with self.assertRaisesRegex(sbom.SbomError, "not an ordinary file"):
                sbom.read_regular_file(link)

    def test_current_repository_graph_generates_and_verifies(self) -> None:
        metadata = sbom.load_cargo_metadata(ROOT)
        lock_data = sbom.read_regular_file(ROOT / "Cargo.lock")
        workspace_versions = {
            package["version"]
            for package in metadata["packages"]
            if package["id"] in set(metadata["workspace_members"])
        }
        self.assertEqual(len(workspace_versions), 1)
        document = sbom.build_sbom(
            metadata,
            lock_data,
            workspace_versions.pop(),
            "0" * 40,
            0,
        )
        rendered = sbom.render_sbom(document)
        reparsed = json.loads(rendered)
        self.assertEqual(reparsed, document)
        self.assertEqual(len(document["packages"]), len(metadata["packages"]) + 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
