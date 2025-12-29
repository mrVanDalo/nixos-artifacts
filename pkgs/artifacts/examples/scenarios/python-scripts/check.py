#!/usr/bin/env python3
"""Check serialization - skip for machine-one/artifact-two, require generation for others."""
import os
import sys

machine = os.environ.get("machine", "")
artifact = os.environ.get("artifact", "")

if not machine or not artifact:
    sys.exit(1)

if machine == "machine-one" and artifact == "artifact-two":
    sys.exit(0)

sys.exit(1)
