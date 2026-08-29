#!/usr/bin/env python3
"""Contract and simulated-install tests for the cross-platform CLI installers."""

from __future__ import annotations

import argparse
import os
import shutil
import stat
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SHELL_INSTALLER = ROOT / "install-cli.sh"
POWERSHELL_INSTALLER = ROOT / "install-cli.ps1"
README = ROOT / "README.md"
WORKFLOW = ROOT / ".github" / "workflows" / "build.yml"


class ShellInstallerHarness:
    def __init__(self) -> None:
        self._temp = tempfile.TemporaryDirectory(prefix="filehub-installer-test-")
        self.root = Path(self._temp.name)
        self.mock_bin = self.root / "mock-bin"
        self.tmp_dir = self.root / "tmp"
        self.install_dir = self.root / "install"
        self.curl_log = self.root / "curl.log"
        self.mock_bin.mkdir()
        self.tmp_dir.mkdir()
        self._write_mock_commands()

    def close(self) -> None:
        self._temp.cleanup()

    def _write_executable(self, name: str, content: str) -> None:
        path = self.mock_bin / name
        path.write_text(content, encoding="utf-8")
        path.chmod(0o755)

    def _write_mock_commands(self) -> None:
        self._write_executable(
            "curl",
            """#!/bin/sh
set -eu
output=""
url=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --output|--header|--proto|--proto-redir)
            value=$2
            [ "$1" = "--output" ] && output=$value
            shift 2
            ;;
        --*) shift ;;
        *) url=$1; shift ;;
    esac
done
printf '%s\n' "$url" >> "$FAKE_CURL_LOG"
case "$url" in
    https://api.github.com/*/releases/latest)
        [ "${FAKE_API_FAIL:-0}" = "0" ] || exit 22
        printf '{"tag_name":"%s"}\n' "${FAKE_API_TAG:-v9.8.7}"
        ;;
    https://github.com/*/releases/download/*)
        [ "${FAKE_DOWNLOAD_FAIL:-0}" = "0" ] || exit 22
        cp "$FAKE_ARCHIVE" "$output"
        ;;
    *) exit 22 ;;
esac
""",
        )
        self._write_executable(
            "uname",
            """#!/bin/sh
case "${1:-}" in
    -s) printf '%s\n' "${FAKE_UNAME_S:-Linux}" ;;
    -m) printf '%s\n' "${FAKE_UNAME_M:-x86_64}" ;;
    *) exit 2 ;;
esac
""",
        )
        self._write_executable(
            "id",
            """#!/bin/sh
if [ "${1:-}" = "-u" ]; then
    printf '1000\n'
else
    exec /usr/bin/id "$@"
fi
""",
        )

    def archive(self, content: bytes = b"filehub-test-binary\n", extra: str | None = None) -> Path:
        archive = self.root / f"archive-{len(list(self.root.glob('archive-*.tar.gz')))}.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            binary = tarfile.TarInfo("filehub")
            binary.mode = 0o755
            binary.size = len(content)
            import io

            bundle.addfile(binary, io.BytesIO(content))
            if extra is not None:
                unexpected = tarfile.TarInfo(extra)
                unexpected.size = 1
                bundle.addfile(unexpected, io.BytesIO(b"x"))
        return archive

    def symlink_archive(self, target: str) -> Path:
        archive = self.root / f"archive-{len(list(self.root.glob('archive-*.tar.gz')))}.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            link = tarfile.TarInfo("filehub")
            link.type = tarfile.SYMTYPE
            link.linkname = target
            bundle.addfile(link)
        return archive

    def run(
        self,
        *arguments: str,
        archive: Path | None = None,
        os_name: str = "Linux",
        architecture: str = "x86_64",
        api_tag: str = "v9.8.7",
        api_fail: bool = False,
        download_fail: bool = False,
        add_install_dir: bool = True,
        from_stdin: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.update(
            {
                "PATH": f"{self.mock_bin}:/usr/bin:/bin",
                "TMPDIR": str(self.tmp_dir),
                "FAKE_CURL_LOG": str(self.curl_log),
                "FAKE_ARCHIVE": str(archive or self.archive()),
                "FAKE_UNAME_S": os_name,
                "FAKE_UNAME_M": architecture,
                "FAKE_API_TAG": api_tag,
                "FAKE_API_FAIL": "1" if api_fail else "0",
                "FAKE_DOWNLOAD_FAIL": "1" if download_fail else "0",
            }
        )
        effective_arguments = list(arguments)
        if add_install_dir:
            effective_arguments.extend(["--install-dir", str(self.install_dir)])
        command = (
            ["sh", "-s", "--", *effective_arguments]
            if from_stdin
            else [str(SHELL_INSTALLER), *effective_arguments]
        )
        script_input = SHELL_INSTALLER.read_text(encoding="utf-8") if from_stdin else None
        return subprocess.run(
            command,
            env=env,
            check=False,
            capture_output=True,
            text=True,
            input=script_input,
        )

    def urls(self) -> list[str]:
        if not self.curl_log.exists():
            return []
        return self.curl_log.read_text(encoding="utf-8").splitlines()


class HarnessTestCase(unittest.TestCase):
    harness: ShellInstallerHarness

    def setUp(self) -> None:
        self.harness = ShellInstallerHarness()

    def tearDown(self) -> None:
        self.harness.close()


class UnitContractTests(HarnessTestCase):
    def test_invalid_and_duplicate_versions_fail_before_network_or_install(self) -> None:
        for arguments in (("latest",), ("1.2",), ("1.2.3.4",), ("1.2.3", "2.0.0")):
            with self.subTest(arguments=arguments):
                result = self.harness.run(*arguments)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((self.harness.install_dir / "filehub").exists())
        self.assertEqual(self.harness.urls(), [])

    def test_unsupported_platform_is_rejected_before_asset_download(self) -> None:
        result = self.harness.run("1.2.3", architecture="aarch64")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported platform Linux/aarch64", result.stderr)
        self.assertEqual(self.harness.urls(), [])

    def test_non_exact_archive_is_rejected_and_existing_binary_is_preserved(self) -> None:
        self.harness.install_dir.mkdir()
        installed = self.harness.install_dir / "filehub"
        installed.write_bytes(b"previous-version")
        result = self.harness.run("1.2.3", archive=self.harness.archive(extra="../unexpected"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exactly one root file", result.stderr)
        self.assertEqual(installed.read_bytes(), b"previous-version")
        self.assertEqual(list(self.harness.tmp_dir.iterdir()), [])

    def test_single_symlink_entry_is_rejected_without_following_its_target(self) -> None:
        self.harness.install_dir.mkdir()
        installed = self.harness.install_dir / "filehub"
        installed.write_bytes(b"previous-version")
        outside = self.harness.root / "outside-target"
        outside.write_bytes(b"must-not-be-installed-or-chmodded")
        outside.chmod(0o600)
        result = self.harness.run("1.2.3", archive=self.harness.symlink_archive(str(outside)))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must be a regular file", result.stderr)
        self.assertEqual(installed.read_bytes(), b"previous-version")
        self.assertEqual(outside.read_bytes(), b"must-not-be-installed-or-chmodded")
        self.assertEqual(stat.S_IMODE(outside.stat().st_mode), 0o600)
        self.assertEqual(list(self.harness.tmp_dir.iterdir()), [])

    def test_invalid_latest_tag_fails_before_asset_download(self) -> None:
        result = self.harness.run(api_tag="latest")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid version", result.stderr)
        self.assertEqual(
            self.harness.urls(),
            ["https://api.github.com/repos/wugren/sfo-filehub/releases/latest"],
        )

    def test_powershell_contract_has_fail_closed_system_and_rollback_branches(self) -> None:
        source = POWERSHELL_INSTALLER.read_text(encoding="utf-8")
        required = (
            "releases/latest",
            "^v?([0-9]+\\.[0-9]+\\.[0-9]+)$",
            "PROCESSOR_ARCHITEW6432",
            "Unsupported Windows architecture",
            "Test-Administrator",
            "ProgramFiles",
            "ArchiveEntries.Count -ne 1",
            "ArchiveListing[0].StartsWith('-')",
            "[IO.FileAttributes]::ReparsePoint",
            "[IO.File]::Replace",
            "GetEnvironmentVariable('Path', 'Machine')",
            "SetEnvironmentVariable('Path', $NewMachinePath, 'Machine')",
            "if (-not $InstallDirWasSpecified)",
            "$PreserveBackup = $true",
            "backup retained at $BackupPath",
            "if (-not $PreserveBackup",
            "finally",
        )
        for token in required:
            self.assertIn(token, source)

    def test_shell_signal_handlers_exit_before_exit_cleanup(self) -> None:
        source = SHELL_INSTALLER.read_text(encoding="utf-8")
        self.assertIn("trap cleanup EXIT", source)
        self.assertIn("trap 'exit 1' HUP INT TERM", source)
        self.assertNotIn("trap cleanup EXIT HUP INT TERM", source)


class DvContractTests(HarnessTestCase):
    def test_stdin_execution_accepts_latest_explicit_version_and_custom_dir(self) -> None:
        latest = self.harness.run(
            archive=self.harness.archive(b"stdin-latest"),
            api_tag="v4.5.6",
            from_stdin=True,
        )
        self.assertEqual(latest.returncode, 0, latest.stderr)
        self.assertEqual((self.harness.install_dir / "filehub").read_bytes(), b"stdin-latest")

        explicit = self.harness.run(
            "4.5.7",
            archive=self.harness.archive(b"stdin-explicit"),
            from_stdin=True,
        )
        self.assertEqual(explicit.returncode, 0, explicit.stderr)
        self.assertEqual((self.harness.install_dir / "filehub").read_bytes(), b"stdin-explicit")
        self.assertEqual(
            self.harness.urls(),
            [
                "https://api.github.com/repos/wugren/sfo-filehub/releases/latest",
                "https://github.com/wugren/sfo-filehub/releases/download/v4.5.6/"
                "filehub-cli_4.5.6_linux-x86_64.tar.gz",
                "https://github.com/wugren/sfo-filehub/releases/download/v4.5.7/"
                "filehub-cli_4.5.7_linux-x86_64.tar.gz",
            ],
        )

    def test_explicit_version_normalizes_tag_and_installs_executable(self) -> None:
        archive = self.harness.archive(b"explicit-version")
        result = self.harness.run("v1.2.3", archive=archive)
        self.assertEqual(result.returncode, 0, result.stderr)
        installed = self.harness.install_dir / "filehub"
        self.assertEqual(installed.read_bytes(), b"explicit-version")
        self.assertTrue(installed.stat().st_mode & stat.S_IXUSR)
        self.assertEqual(
            self.harness.urls(),
            [
                "https://github.com/wugren/sfo-filehub/releases/download/v1.2.3/"
                "filehub-cli_1.2.3_linux-x86_64.tar.gz"
            ],
        )

    def test_missing_version_resolves_latest_tag_then_downloads_matching_asset(self) -> None:
        result = self.harness.run(archive=self.harness.archive(b"latest"), api_tag="v2.4.6")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.harness.install_dir / "filehub").read_bytes(), b"latest")
        self.assertEqual(
            self.harness.urls(),
            [
                "https://api.github.com/repos/wugren/sfo-filehub/releases/latest",
                "https://github.com/wugren/sfo-filehub/releases/download/v2.4.6/"
                "filehub-cli_2.4.6_linux-x86_64.tar.gz",
            ],
        )

    def test_macos_arm64_selects_the_existing_release_asset(self) -> None:
        result = self.harness.run(
            "3.2.1",
            archive=self.harness.archive(b"macos"),
            os_name="Darwin",
            architecture="arm64",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.harness.install_dir / "filehub").read_bytes(), b"macos")
        self.assertEqual(
            self.harness.urls(),
            [
                "https://github.com/wugren/sfo-filehub/releases/download/v3.2.1/"
                "filehub-cli_3.2.1_macos-aarch64.tar.gz"
            ],
        )

    def test_reinstall_replaces_existing_binary_without_staging_residue(self) -> None:
        first = self.harness.run("1.0.0", archive=self.harness.archive(b"first"))
        second = self.harness.run("1.0.1", archive=self.harness.archive(b"second"))
        self.assertEqual(first.returncode, 0, first.stderr)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual((self.harness.install_dir / "filehub").read_bytes(), b"second")
        self.assertEqual(list(self.harness.install_dir.glob(".filehub.install.*")), [])

    def test_api_and_download_failures_preserve_existing_binary_and_cleanup(self) -> None:
        self.harness.install_dir.mkdir()
        installed = self.harness.install_dir / "filehub"
        installed.write_bytes(b"existing")
        for options in ({"api_fail": True}, {"download_fail": True}):
            with self.subTest(options=options):
                result = self.harness.run(archive=self.harness.archive(), **options)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(installed.read_bytes(), b"existing")
                self.assertEqual(list(self.harness.tmp_dir.iterdir()), [])


class IntegrationContractTests(HarnessTestCase):
    def test_release_asset_names_match_workflow_and_both_installers(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        shell = SHELL_INSTALLER.read_text(encoding="utf-8")
        powershell = POWERSHELL_INSTALLER.read_text(encoding="utf-8")
        for platform in ("linux-x86_64", "macos-aarch64", "windows-x86_64"):
            self.assertIn(platform, workflow)
        self.assertIn('archive_name="filehub-cli_${version}_${platform}.tar.gz"', shell)
        self.assertIn('$ArchiveName = "filehub-cli_${ResolvedVersion}_windows-x86_64.tar.gz"', powershell)
        self.assertIn('archive_entries" = "filehub"', shell)
        self.assertIn("ArchiveEntries[0] -cne 'filehub.exe'", powershell)

    def test_readme_documents_latest_explicit_custom_uninstall_and_manual_fallback(self) -> None:
        readme = README.read_text(encoding="utf-8")
        required = (
            "bash -o pipefail -c 'curl -fsSL https://raw.githubusercontent.com/wugren/sfo-filehub/main/install-cli.sh | sh'",
            "| sh -s -- 0.1.0",
            "| sh -s -- --install-dir \"$HOME/.local/bin\"",
            "Invoke-RestMethod -Uri 'https://raw.githubusercontent.com/wugren/sfo-filehub/main/install-cli.ps1'",
            "[scriptblock]::Create",
            "))) -Version 0.1.0",
            '-InstallDir "$HOME\\bin"',
            "/usr/local/bin/filehub",
            "%ProgramFiles%\\filehub\\bin\\filehub.exe",
            "sudo rm /usr/local/bin/filehub",
            "会立即执行 `wugren/sfo-filehub` 仓库 `main` 分支中的脚本",
            "脚本不可用时，也可以手工下载",
        )
        for token in required:
            self.assertIn(token, readme)

    def test_shell_help_matches_documented_public_interface(self) -> None:
        result = self.harness.run("--help", add_install_dir=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("[VERSION] [--install-dir DIR]", result.stdout)
        self.assertIn("latest stable", result.stdout)

    def test_powershell_script_parses_when_pwsh_is_available(self) -> None:
        pwsh = shutil.which("pwsh")
        if pwsh is None:
            self.skipTest("pwsh is unavailable; Windows execution remains a target-platform gap")
        command = (
            "$ErrorActionPreference='Stop'; "
            f"[void][scriptblock]::Create((Get-Content -Raw '{POWERSHELL_INSTALLER}'))"
        )
        result = subprocess.run(
            [pwsh, "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", command],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)


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
