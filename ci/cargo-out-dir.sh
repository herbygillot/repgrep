#!/bin/bash

# Finds Cargo's `OUT_DIR` directory from the most recent build.
#
# This requires one parameter corresponding to the target directory
# to search for the build output.

if [ $# != 1 ]; then
  echo "Usage: $(basename "$0") <target-dir>" >&2
  exit 2
fi

# This works by finding the most recent stamp file, which is produced by
# in the build.rs build script.
target_dir="$1"
find "$target_dir" -name repgrep-stamp -print0 \
  | xargs -0 ls -t \
  | head -n1 \
  | xargs dirname
