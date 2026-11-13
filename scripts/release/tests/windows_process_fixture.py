"""Short-lived native children for the PowerShell process-runner tests."""

import json
import os
from pathlib import Path
import subprocess
import sys
import time


sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")
mode = sys.argv[1]
if mode == "arguments":
    print(json.dumps(sys.argv[2:], ensure_ascii=False))
elif mode == "streams":
    sys.stdout.write("O" * 200_000)
    sys.stdout.flush()
    sys.stderr.write("E" * 200_000)
    sys.stderr.flush()
elif mode == "failure":
    print("stdout failure detail", flush=True)
    print("stderr failure detail", file=sys.stderr, flush=True)
    sys.exit(7)
elif mode == "progress":
    if not Path(sys.argv[2]).exists():
        sys.exit("start progress was not delivered before child execution")
    print("progress reached caller")
elif mode == "tree":
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(20)"])
    try:
        Path(sys.argv[2]).write_text(json.dumps([os.getpid(), child.pid]), encoding="utf-8")
        print("parent and child started", flush=True)
        time.sleep(8)
    finally:
        child.kill()
        child.wait(timeout=3)
elif mode == "held-pipe":
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(20)"])
    Path(sys.argv[2]).write_text(str(child.pid), encoding="utf-8")
    # Exit while the test-owned child retains both inherited output pipes.
else:
    raise ValueError(f"unknown process fixture mode: {mode}")
