#!/usr/bin/env python3
"""Static contracts for Docker's mounted YAML configuration boundary."""

from __future__ import annotations

import argparse
import subprocess
import unittest
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[1]
DOCKERFILE = (ROOT / "docker" / "Dockerfile").read_text(encoding="utf-8")
ENTRYPOINT = (ROOT / "docker" / "entrypoint.sh").read_text(encoding="utf-8")
NGINX = (ROOT / "docker" / "nginx.conf").read_text(encoding="utf-8")
DOCKER_README = (ROOT / "docker" / "README.md").read_text(encoding="utf-8")
ROOT_README = (ROOT / "README.md").read_text(encoding="utf-8")
EXAMPLE_PATH = ROOT / "docker" / "filehub-server.example.yaml"


class UnitContractTests(unittest.TestCase):
    def test_docker_example_uses_fixed_runtime_paths_and_port(self) -> None:
        config = yaml.safe_load(EXAMPLE_PATH.read_text(encoding="utf-8"))
        self.assertEqual(config["server"]["server_addr"], "127.0.0.1")
        self.assertEqual(config["server"]["port"], 8080)
        self.assertEqual(config["files"]["data_dir"], "/data/files")
        self.assertEqual(config["db_path"], "/data/filehub.db")
        self.assertIn("BEGIN PRIVATE KEY", config["users"]["session_private_key"])
        self.assertTrue(config["users"]["users"])

    def test_entrypoint_requires_one_fixed_config_without_env_generation(self) -> None:
        self.assertIn("CONFIG_PATH=/etc/filehub/filehub-server.yaml", ENTRYPOINT)
        self.assertIn('[ ! -f "$CONFIG_PATH" ]', ENTRYPOINT)
        self.assertIn('[ ! -r "$CONFIG_PATH" ]', ENTRYPOINT)
        self.assertIn('/usr/local/bin/filehub-server "$CONFIG_PATH"', ENTRYPOINT)
        self.assertNotIn("FH_", ENTRYPOINT)
        self.assertNotIn("jq", ENTRYPOINT)
        self.assertNotIn("openssl", ENTRYPOINT)

    def test_runtime_image_does_not_install_config_generators(self) -> None:
        self.assertNotIn("jq", DOCKERFILE)
        self.assertNotIn("openssl", DOCKERFILE)
        self.assertIn("COPY nginx.conf /etc/nginx/conf.d/filehub.conf", DOCKERFILE)
        self.assertNotIn("filehub.conf.tpl", DOCKERFILE)


class DvContractTests(unittest.TestCase):
    def test_nginx_and_example_share_fixed_internal_port(self) -> None:
        self.assertNotIn("__SERVER_PORT__", NGINX)
        self.assertEqual(NGINX.count("proxy_pass http://127.0.0.1:8080;"), 4)
        config = yaml.safe_load(EXAMPLE_PATH.read_text(encoding="utf-8"))
        self.assertEqual(config["server"], {
            **config["server"],
            "server_addr": "127.0.0.1",
            "port": 8080,
        })

    def test_entrypoint_does_not_write_or_replace_config(self) -> None:
        forbidden = (
            '> "$CONFIG_PATH"',
            "> '$CONFIG_PATH'",
            "sed -i",
            "cp " + '"$CONFIG_PATH"',
            "mv " + '"$CONFIG_PATH"',
        )
        for token in forbidden:
            self.assertNotIn(token, ENTRYPOINT)
        self.assertIn("filehub-server exited with status", ENTRYPOINT)
        self.assertIn('while kill -0 "$SERVER_PID"', ENTRYPOINT)
        self.assertIn('kill -0 "$NGINX_PID"', ENTRYPOINT)

    def test_documented_runs_use_read_only_config_mount(self) -> None:
        for text in (DOCKER_README, ROOT_README):
            self.assertIn("dst=/etc/filehub/filehub-server.yaml,readonly", text)
            self.assertNotIn("-e FH_", text)
        self.assertIn("read_only: true", DOCKER_README)


class IntegrationContractTests(unittest.TestCase):
    def test_entrypoint_has_valid_posix_shell_syntax(self) -> None:
        result = subprocess.run(
            ["sh", "-n", str(ROOT / "docker" / "entrypoint.sh")],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_required_docker_assets_exist_and_are_nonempty(self) -> None:
        for relative in (
            "docker/Dockerfile",
            "docker/entrypoint.sh",
            "docker/nginx.conf",
            "docker/filehub-server.example.yaml",
        ):
            path = ROOT / relative
            self.assertTrue(path.is_file(), relative)
            self.assertGreater(path.stat().st_size, 0, relative)


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
