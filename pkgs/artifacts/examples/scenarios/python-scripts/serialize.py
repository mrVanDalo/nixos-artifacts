#!/usr/bin/env python3
"""Test serialization - copy files to test output directory."""
import os
import shutil
import sys

test_output_dir = os.environ.get("ARTIFACTS_TEST_OUTPUT_DIR")
if test_output_dir:
    artifact_context = os.environ.get("artifact_context", "nixos")
    artifact = os.environ.get("artifact", "unknown")
    out = os.environ.get("out", "")

    if artifact_context == "homemanager":
        username = os.environ.get("username", "unknown")
        target_dir = os.path.join(test_output_dir, "users", username, artifact)
    else:
        machine = os.environ.get("machine", "unknown")
        target_dir = os.path.join(test_output_dir, "machines", machine, artifact)

    os.makedirs(target_dir, exist_ok=True)
    for item in os.listdir(out):
        src = os.path.join(out, item)
        dst = os.path.join(target_dir, item)
        if os.path.isfile(src):
            shutil.copy2(src, dst)
        else:
            shutil.copytree(src, dst)

sys.exit(0)
