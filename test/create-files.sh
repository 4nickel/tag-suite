#!/bin/bash

echo "Creating test files"

mkdir -p "${1}/1500"
for n in $(seq 1 1 1500)
do
    s=$(printf '%04d' ${n})
    echo "${s}" >> "${1}/1500/${s}"
done

echo "a" > "${1}/a"
echo "b" > "${1}/b"
echo "c" > "${1}/c"
