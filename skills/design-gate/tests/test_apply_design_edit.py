import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "apply-design-edit.py"


class DesignEditTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.scope = self.root / "design"
        self.module = self.scope / "rule-module"
        self.scope.mkdir()
        self.module.mkdir()

    def tearDown(self):
        self.temporary.cleanup()

    def run_edit(self, edits):
        return subprocess.run(
            [
                "python3",
                str(SCRIPT),
                "--scope",
                str(self.scope),
                "--module",
                str(self.module),
            ],
            input=json.dumps({"edits": edits}),
            capture_output=True,
            text=True,
            check=False,
        )

    def test_writes_a_design_file_in_scope(self):
        result = self.run_edit([{"path": "model/connection.md", "content": "proposal"}])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.scope / "model/connection.md").read_text(), "proposal")

    def test_rejects_rule_module_write_before_applying_any_edit(self):
        result = self.run_edit(
            [
                {"path": "model/connection.md", "content": "proposal"},
                {"path": "rule-module/manifest.yaml", "content": "changed"},
            ]
        )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)["code"], "rules_modified")
        self.assertFalse((self.scope / "model/connection.md").exists())
        self.assertFalse((self.module / "manifest.yaml").exists())

    def test_rejects_parent_traversal(self):
        result = self.run_edit([{"path": "../outside.md", "content": "proposal"}])
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)["code"], "rules_modified")
        self.assertFalse((self.root / "outside.md").exists())

    def test_rejects_symlink_escape(self):
        outside = self.root / "outside"
        outside.mkdir()
        (self.scope / "escape").symlink_to(outside, target_is_directory=True)
        result = self.run_edit([{"path": "escape/new.md", "content": "proposal"}])
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)["code"], "rules_modified")
        self.assertFalse((outside / "new.md").exists())


if __name__ == "__main__":
    unittest.main()
