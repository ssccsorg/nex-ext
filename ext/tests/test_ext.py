"""Unit tests for the nex-ext runner harness (standard library only).

Run from the repository root:

    PYTHONPATH=ext python3 -m unittest discover -s ext/tests -t ext -v

No network, Docker, or external services are touched.
"""

import sys
import tempfile
import unittest
from pathlib import Path

EXT_ROOT = Path(__file__).resolve().parents[1]
if str(EXT_ROOT) not in sys.path:
    sys.path.insert(0, str(EXT_ROOT))

from _runner.base import AbstractEngine  # noqa: E402
from _runner.cli import _build_parser, _find_engine  # noqa: E402
from _runner.engines import ALL as ENGINE_REGISTRY  # noqa: E402
from _runner.checks import detect_embedding_dimension  # noqa: E402

EXPECTED_ENGINES = {"lightrag", "edgequake", "graphiti", "memgraph"}

# Engines whose image is built from a local Dockerfile in this repository.
DOCKERFILE_ENGINES = {"lightrag", "graphiti", "memgraph"}


class EngineRegistryTests(unittest.TestCase):
    def test_registry_matches_expected_engines(self):
        self.assertEqual(set(ENGINE_REGISTRY), EXPECTED_ENGINES)

    def test_all_engines_implement_the_contract(self):
        for name, cls in ENGINE_REGISTRY.items():
            with self.subTest(engine=name):
                self.assertTrue(issubclass(cls, AbstractEngine))
                self.assertIsInstance(cls.name, property)


class EngineAssetTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def _construct(self, name, base):
        return ENGINE_REGISTRY[name](base)

    def test_tunnel_config_is_an_absolute_existing_path(self):
        for name in EXPECTED_ENGINES:
            with self.subTest(engine=name):
                # Production scripts set EXT_DIR to the real ext/ root, so
                # asset paths resolve under the repository, not a temp dir.
                config = Path(self._construct(name, str(EXT_ROOT)).tunnel_config)
                self.assertTrue(config.is_absolute())
                self.assertTrue(config.exists(), f"missing tunnel config: {config}")

    def test_docker_build_assets_exist(self):
        for name in DOCKERFILE_ENGINES:
            with self.subTest(engine=name):
                engine = self._construct(name, str(EXT_ROOT))
                if name == "memgraph":
                    # MemgraphEngine builds the proxy image from a Dockerfile
                    # in its engine dir; it stores no `dockerfile` attribute.
                    dockerfile = Path(engine.proxy_dir) / "Dockerfile"
                else:
                    dockerfile = Path(engine.dockerfile)
                self.assertTrue(
                    dockerfile.exists(),
                    f"missing Dockerfile: {dockerfile}",
                )
        engine = self._construct("edgequake", str(EXT_ROOT))
        self.assertTrue(Path(engine.compose_file).exists())

    def test_constructors_are_side_effect_free(self):
        for name in EXPECTED_ENGINES:
            with self.subTest(engine=name):
                self.assertIsInstance(self._construct(name, self.tmp).name, str)


class CliTests(unittest.TestCase):
    def test_parser_defaults_to_an_engine_in_the_registry(self):
        parser = _build_parser()
        default = parser.parse_args([]).engine
        self.assertIn(default, ENGINE_REGISTRY)

    def test_unknown_engine_exits(self):
        with self.assertRaises(SystemExit):
            _find_engine("does-not-exist")

    def test_known_engine_resolves(self):
        engine = _find_engine("memgraph")
        self.assertEqual(engine.name, "memgraph")


class EmbeddingDimTests(unittest.TestCase):
    def test_env_override_skips_network(self):
        # A valid override returns immediately without probing the API.
        self.assertEqual(
            detect_embedding_dimension("http://127.0.0.1:1", "m", env_override="512"),
            512,
        )


if __name__ == "__main__":
    unittest.main()
