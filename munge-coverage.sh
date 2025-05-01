#!/bin/bash
set -euo pipefail

ALL_FILES=$(find . -name "*.go")
IGNORE_FILES=$(grep -l "/// coverage-ignore" ${ALL_FILES})
echo IGNORE_FILES:${IGNORE_FILES}

if [[ "" != "${IGNORE_FILES}" ]]; then
    echo "Ignoring some files from coverage:" ${IGNORE_FILES}
    grep -v "${IGNORE_FILES}" .coverage.out.tmp > .coverage.out
else
    echo "Including all files in coverage"
    cp .coverage.out.tmp .coverage.out
fi
