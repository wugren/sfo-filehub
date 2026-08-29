#!/usr/bin/env python3
"""Static contract tests for the filehub GitHub Actions build/release workflow."""

from __future__ import annotations

import argparse
import re
import subprocess
import unittest
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "build.yml"
WORKFLOW_TEXT = WORKFLOW_PATH.read_text(encoding="utf-8")
WORKFLOW = yaml.load(WORKFLOW_TEXT, Loader=yaml.BaseLoader)
ROOT_README = (ROOT / "README.md").read_text(encoding="utf-8")


def job(name: str) -> dict:
    return WORKFLOW["jobs"][name]


def steps(name: str) -> list[dict]:
    return job(name)["steps"]


def step(name: str, title: str) -> dict:
    matches = [item for item in steps(name) if item.get("name") == title]
    if len(matches) != 1:
        raise AssertionError(f"expected one {name!r} step {title!r}, got {len(matches)}")
    return matches[0]


def needs(name: str) -> set[str]:
    value = job(name).get("needs", [])
    return {value} if isinstance(value, str) else set(value)


class UnitContractTests(unittest.TestCase):
    def test_dispatch_inputs_are_explicit_and_safe_by_default(self) -> None:
        inputs = WORKFLOW["on"]["workflow_dispatch"]["inputs"]
        self.assertEqual(inputs["publish"]["type"], "boolean")
        self.assertEqual(inputs["publish"]["default"], "false")
        self.assertEqual(inputs["release_tag"]["type"], "string")
        self.assertEqual(inputs["release_tag"]["default"], "")

    def test_release_runs_are_serialized_without_cancellation(self) -> None:
        concurrency = WORKFLOW["concurrency"]
        self.assertEqual(
            concurrency["group"],
            "release-${{ inputs.release_tag || github.ref_name }}",
        )
        self.assertEqual(concurrency["cancel-in-progress"], "false")

    def test_cargo_update_runs_exactly_once_before_version_resolution(self) -> None:
        version_steps = steps("version")
        toolchain_index = next(
            index for index, item in enumerate(version_steps)
            if item.get("name") == "Install Rust toolchain for dependency resolution"
        )
        update_index = next(
            index for index, item in enumerate(version_steps)
            if item.get("name") == "Update Rust dependencies and resolve version gates"
        )
        update_step = step("version", "Update Rust dependencies and resolve version gates")
        script = update_step["run"]
        self.assertLess(toolchain_index, update_index)
        self.assertEqual(len(re.findall(r"(?m)^\s*cargo update\s*$", WORKFLOW_TEXT)), 1)
        self.assertLess(script.index("cargo update"), script.index("cargo metadata"))
        self.assertIn('test -s Cargo.lock', script)

    def test_manual_publish_gate_rejects_invalid_requests(self) -> None:
        script = step("version", "Update Rust dependencies and resolve version gates")["run"]
        required = (
            '"${DISPATCH_PUBLISH}" == "true"',
            'release_tag is required when publish=true',
            '^v[0-9]+\\.[0-9]+\\.[0-9]+$',
            '"${release_tag}" != "v${version}"',
            'Manual publication is allowed only in',
            'publish_requested=true',
        )
        for token in required:
            self.assertIn(token, script)

    def test_every_rust_command_uses_downloaded_lockfile_first(self) -> None:
        build_steps = steps("build")
        download_index = next(
            index for index, item in enumerate(build_steps)
            if item.get("name") == "Download updated Cargo.lock"
        )
        install_index = next(
            index for index, item in enumerate(build_steps)
            if item.get("name") == "Install updated Cargo.lock"
        )
        self.assertLess(download_index, install_index)
        install_script = build_steps[install_index]["run"]
        self.assertIn("cp .ci-cargo-lock/Cargo.lock Cargo.lock", install_script)
        cargo_indexes = [
            index for index, item in enumerate(build_steps)
            if re.search(r"(?m)^\s*cargo (?:test|build)\b", item.get("run", ""))
        ]
        self.assertTrue(cargo_indexes)
        self.assertTrue(all(install_index < index for index in cargo_indexes))
        self.assertNotIn("cargo update", "\n".join(item.get("run", "") for item in build_steps))
        locked_commands = [
            line.strip()
            for job_name in WORKFLOW["jobs"]
            for item in steps(job_name)
            for line in item.get("run", "").splitlines()
            if re.search(r"(?:^\s*cargo|\$\(cargo) (?:metadata|test|build)\b", line)
        ]
        self.assertTrue(locked_commands)
        self.assertTrue(
            all("--locked" in command for command in locked_commands),
            locked_commands,
        )


class DvContractTests(unittest.TestCase):
    def test_updated_lockfile_is_uploaded_and_consumed_by_matrix(self) -> None:
        upload = step("version", "Store updated Cargo.lock")
        download = step("build", "Download updated Cargo.lock")
        self.assertEqual(upload["with"]["name"], "cargo-lock")
        self.assertEqual(upload["with"]["path"], "Cargo.lock")
        self.assertEqual(download["with"]["name"], "cargo-lock")
        self.assertEqual(download["with"]["path"], ".ci-cargo-lock")
        self.assertEqual(needs("build"), {"version"})

    def test_publication_authorization_waits_for_builds_and_binds_tag_sha(self) -> None:
        self.assertEqual(needs("authorize-publication"), {"version", "build", "test-web"})
        authorize = step("authorize-publication", "Resolve and authorize release tag")["run"]
        self.assertIn("refs/tags/${RELEASE_TAG}^{commit}", authorize)
        self.assertIn('"${tag_sha}" != "${SOURCE_SHA}"', authorize)
        self.assertIn('echo "publish=${publish}"', authorize)
        self.assertIn('echo "release_tag=${release_tag}"', authorize)

    def test_build_only_path_produces_no_publish_output(self) -> None:
        authorize = step("authorize-publication", "Resolve and authorize release tag")["run"]
        self.assertLess(authorize.index("publish=false"), authorize.index("if [["))
        self.assertIn('if [[ "${PUBLISH_REQUESTED}" == "true" ]]', authorize)
        self.assertLess(authorize.index("publish=true"), authorize.index('echo "publish=${publish}"'))

    def test_publish_consumers_use_only_authorized_outputs(self) -> None:
        image = job("build-image")
        release = job("release")
        expected_if = "${{ needs.authorize-publication.outputs.publish == 'true' }}"
        self.assertEqual(step("build-image", "Publish image to GHCR")["if"], expected_if)
        self.assertEqual(release["if"], expected_if)
        self.assertIn("authorize-publication", needs("build-image"))
        self.assertIn("authorize-publication", needs("release"))
        self.assertEqual(
            step("release", "Create or update GitHub Release")["env"]["RELEASE_TAG"],
            "${{ needs.authorize-publication.outputs.release_tag }}",
        )
        self.assertEqual(
            step("release", "Create or update GitHub Release")["env"]["SOURCE_SHA"],
            "${{ needs.version.outputs.source_sha }}",
        )

    def test_release_title_uses_version_and_existing_draft_is_published(self) -> None:
        publish = step("release", "Create or update GitHub Release")["run"]
        branches = re.search(
            r"if gh release view .*?; then\n(?P<existing>.*?)\n\s*else\n"
            r"(?P<new>.*?)\n\s*fi\s*$",
            publish,
            re.DOTALL,
        )
        self.assertIsNotNone(branches)
        existing = branches.group("existing")
        new = branches.group("new")

        self.assertLess(existing.index("gh release upload"), existing.index("git fetch"))
        self.assertLess(existing.index("git fetch"), existing.index("gh release edit"))
        self.assertIn("refs/tags/${RELEASE_TAG}^{commit}", existing)
        self.assertIn('"${tag_sha}" != "${SOURCE_SHA}"', existing)
        self.assertIn('gh release edit "${RELEASE_TAG}"', existing)
        self.assertIn('--title "${VERSION}"', existing)
        self.assertIn("--draft=false", existing)

        self.assertIn('gh release create "${RELEASE_TAG}"', new)
        self.assertIn('--title "${VERSION}"', new)
        self.assertNotIn("--draft", new)
        self.assertNotIn('--title "filehub ${VERSION}"', publish)
        self.assertNotIn('--title "${RELEASE_TAG}"', publish)

    def test_image_publish_pushes_version_and_latest_from_same_image(self) -> None:
        publish = step("build-image", "Publish image to GHCR")["run"]
        push_scripts = [
            item["run"]
            for job_name in WORKFLOW["jobs"]
            for item in steps(job_name)
            if "docker push" in item.get("run", "")
        ]
        self.assertEqual(push_scripts, [publish])
        required = (
            'version_image="ghcr.io/${owner}/filehub:v${VERSION}"',
            'latest_image="ghcr.io/${owner}/filehub:latest"',
            'docker tag "$version_image" "$latest_image"',
            'docker push "$version_image"',
            'docker push "$latest_image"',
            'test "$latest_digest" = "$version_digest"',
        )
        for token in required:
            self.assertIn(token, publish)
        self.assertLess(
            publish.index('docker tag "$version_image" "$latest_image"'),
            publish.index('docker push "$latest_image"'),
        )

    def test_image_smoke_uses_read_only_yaml_and_rejects_missing_config(self) -> None:
        smoke = step("build-image", "Smoke test container startup")["run"]
        required = (
            'config="${GITHUB_WORKSPACE}/docker/filehub-server.example.yaml"',
            "dst=/etc/filehub/filehub-server.yaml,readonly",
            'config_hash_before="$(sha256sum "$config"',
            'test "$config_hash_after" = "$config_hash_before"',
            'if docker run --rm "$image"; then',
            "container without mounted config unexpectedly started",
        )
        for token in required:
            self.assertIn(token, smoke)
        self.assertNotIn("-e FH_", smoke)


class IntegrationContractTests(unittest.TestCase):
    def test_all_run_scripts_have_valid_bash_syntax(self) -> None:
        for job_name in WORKFLOW["jobs"]:
            for item in steps(job_name):
                script = item.get("run")
                if script is None:
                    continue
                result = subprocess.run(
                    ["bash", "-n", "-c", script],
                    check=False,
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(
                    result.returncode,
                    0,
                    f"{job_name}/{item.get('name')}: {result.stderr}",
                )

    def test_all_downstream_checkouts_use_the_same_source_sha(self) -> None:
        for name in ("build", "test-web", "authorize-publication", "build-image", "release"):
            checkout = next(item for item in steps(name) if item.get("uses", "").startswith("actions/checkout@"))
            self.assertEqual(checkout["with"]["ref"], "${{ needs.version.outputs.source_sha }}")

    def test_each_external_write_rechecks_release_tag(self) -> None:
        image_script = step("build-image", "Publish image to GHCR")["run"]
        release_script = step("release", "Verify release tag still points at built source")["run"]
        for script in (image_script, release_script):
            self.assertIn("git fetch --force --no-tags origin", script)
            self.assertIn("refs/tags/${RELEASE_TAG}^{commit}", script)
            self.assertIn('"${tag_sha}" != "${SOURCE_SHA}"', script)

    def test_release_publishes_only_cli_archives(self) -> None:
        release_step_names = {item.get("name") for item in steps("release")}
        self.assertNotIn("Download server binary", release_step_names)
        self.assertNotIn("Download admin-web dist", release_step_names)
        self.assertNotIn("Package server and admin-web release archive", release_step_names)
        cli_download = step("release", "Download CLI archives")
        self.assertEqual(cli_download["with"]["pattern"], "filehub-cli-*")

        verify = step("release", "Verify release assets")["run"]
        expected = (
            "filehub-cli_${VERSION}_linux-x86_64.tar.gz",
            "filehub-cli_${VERSION}_macos-aarch64.tar.gz",
            "filehub-cli_${VERSION}_windows-x86_64.tar.gz",
        )
        for asset in expected:
            self.assertIn(asset, verify)
        self.assertNotIn("filehub-server_${VERSION}", verify)

        publish = step("release", "Create or update GitHub Release")["run"]
        self.assertIn("Expected exactly three release archives", publish)
        self.assertNotIn("filehub-server_${VERSION}", publish)
        self.assertIn("Docker image: ghcr.io/%s/filehub:v%s", publish)
        for asset in (
            "filehub-cli_%s_linux-x86_64.tar.gz",
            "filehub-cli_%s_macos-aarch64.tar.gz",
            "filehub-cli_%s_windows-x86_64.tar.gz",
        ):
            self.assertIn(asset, publish)
        self.assertIn("gh release create", publish)
        self.assertIn("gh release upload", publish)

    def test_docker_image_still_consumes_server_and_web_artifacts(self) -> None:
        server_download = step("build-image", "Download server binary")
        web_download = step("build-image", "Download admin-web dist")
        self.assertEqual(server_download["with"]["name"], "filehub-server")
        self.assertEqual(server_download["with"]["path"], "ctx/server")
        self.assertEqual(web_download["with"]["name"], "web-dist")
        self.assertEqual(web_download["with"]["path"], "ctx/web")

        assemble = step("build-image", "Assemble minimal image context")["run"]
        self.assertIn("chmod +x ctx/server/filehub-server", assemble)
        self.assertIn("test -f ctx/web/index.html", assemble)

    def test_readme_matches_cli_only_release_contract(self) -> None:
        self.assertNotIn("filehub-server_<版本>_linux_x86_64.tar.gz", ROOT_README)
        self.assertIn("GitHub Release 只发布 CLI 文件", ROOT_README)
        for artifact in (
            "filehub-cli_<版本>_linux-x86_64.tar.gz",
            "filehub-cli_<版本>_macos-aarch64.tar.gz",
            "filehub-cli_<版本>_windows-x86_64.tar.gz",
        ):
            self.assertIn(artifact, ROOT_README)

    def test_job_dependency_graph_is_acyclic(self) -> None:
        graph = {name: needs(name) for name in WORKFLOW["jobs"]}
        visited: set[str] = set()
        active: set[str] = set()

        def visit(name: str) -> None:
            if name in visited:
                return
            self.assertNotIn(name, active, f"job dependency cycle at {name}")
            active.add(name)
            for dependency in graph[name]:
                self.assertIn(dependency, graph)
                visit(dependency)
            active.remove(name)
            visited.add(name)

        for name in graph:
            visit(name)


SUITES = {
    "unit": UnitContractTests,
    "dv": DvContractTests,
    "integration": IntegrationContractTests,
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--suite", choices=sorted(SUITES), required=True)
    args = parser.parse_args()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(SUITES[args.suite])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
