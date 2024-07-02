# VLC In TEE

## Run method

### Prepare environment

First step, you could refer to [Run in TEE](./README.md#run-in-tee) for preparing environment.

### Run VLC TEE Images

```bash
cd image
cargo run --bin run-solo-vlc-enclave -- . --features nitro-enclaves
```

## Testing

```bash
cargo run --bin call_vlc_client --features nitro-enclaves
```

