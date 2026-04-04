import sys
import os
import site
import subprocess
import tempfile

sys.path.append(os.path.join(os.path.dirname(__file__), 'helpers'))

PYVIPS_REQUIREMENT = "pyvips==3.1.1"


def _pydeps_dir():
    root = os.environ.get("XDG_CACHE_HOME")
    if not root:
        home = os.path.expanduser("~")
        root = os.path.join(home, ".cache") if home != "~" else tempfile.gettempdir()

    pyver = f"py{sys.version_info.major}{sys.version_info.minor}"
    return os.path.join(root, "libvips-test-suite", pyver)


def _install_pyvips(target):
    env = os.environ.copy()
    env.setdefault("PIP_DISABLE_PIP_VERSION_CHECK", "1")

    try:
        subprocess.run(
            [sys.executable, "-m", "pip", "--version"],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )
    except subprocess.CalledProcessError:
        subprocess.run(
            [sys.executable, "-m", "ensurepip", "--upgrade"],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )

    subprocess.run(
        [sys.executable, "-m", "pip", "install", "--upgrade", "--target", target,
         PYVIPS_REQUIREMENT],
        check=True,
        env=env,
    )


def _ensure_pyvips():
    try:
        import pyvips  # noqa: F401
        return
    except ModuleNotFoundError:
        pass

    deps = _pydeps_dir()
    os.makedirs(deps, exist_ok=True)
    site.addsitedir(deps)

    try:
        import pyvips  # noqa: F401
        return
    except ModuleNotFoundError:
        _install_pyvips(deps)
        import pyvips  # noqa: F401


_ensure_pyvips()
