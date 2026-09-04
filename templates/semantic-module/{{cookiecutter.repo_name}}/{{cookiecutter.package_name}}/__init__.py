"""{{ cookiecutter.description }}

The package exists so the module payload — manifest, emitted schemas, skeletons —
ships in a Python distribution as well as an npm one. It carries no logic: the
module is data, and the tools that read it live elsewhere.
"""

import pathlib

MODULE_ROOT = pathlib.Path(__file__).resolve().parent
MANIFEST_PATH = MODULE_ROOT / "manifest.yaml"


def module_root() -> pathlib.Path:
    """The directory a Filament tool loads this module from."""
    return MODULE_ROOT
