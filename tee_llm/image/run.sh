#!/usr/bin/env bash

docker build . -t llama
nitro-cli build-enclave --docker-uri llama:latest --output-file llama-tee.eif

if [ "$1" = "debug" ]; then
    nitro-cli run-enclave --cpu-count 4 --memory 16384  --enclave-cid 15 --eif-path llama-tee.eif --attach-console
else
    nitro-cli run-enclave --cpu-count 4 --memory 16384  --enclave-cid 15 --eif-path llama-tee.eif
fi