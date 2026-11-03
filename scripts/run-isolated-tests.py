"""Run a Windows test executable with explicit pipe handles and isolated data."""
import os
import pathlib
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parents[1]
binary = pathlib.Path(sys.argv[1]).resolve()
label = sys.argv[2]
output = root / "verification" / label
output.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env["ZSCLIP_DATA_DIR"] = str(output / "profile")
env["ZSCLIP_TEST_TEMP_ROOT"] = str(output / "tmp")
env.setdefault("ZSCLIP_TEST_UI_LANGUAGE", "en")
pathlib.Path(env["ZSCLIP_TEST_TEMP_ROOT"]).mkdir(exist_ok=True)
result = subprocess.run([str(binary), *sys.argv[3:]], cwd=root, env=env,
                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
(output / "tests.log").write_bytes(result.stdout)
decoded = result.stdout.decode("utf-8", "replace")
print(decoded[-10000:] if result.returncode else "\n".join(
    line for line in decoded.splitlines() if line.startswith("test result:") or "VV hidden paint:" in line))
print(f"exit={result.returncode}; log={output / 'tests.log'}")
sys.exit(result.returncode)
