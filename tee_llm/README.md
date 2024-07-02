# Llm In TEE

This submodule is providing two running methods for your selecting.

## Run mode

### Run once per request

Different large model can be converted into different tee format file images and put on disk. 

When a request comes, it determines which image to load and executes it once and exits according to the model parameters.

- Advantages: It does not occupy memory and CPU resources for a long time
- Disadvantages: TEE is required every time, model initialization time, and more latency.

### Often in the TEE backend process

A specific large model in tee listens for requests and processes them.

- Advantages: It saves TEE initialization time and model loading initialization time
- Disadvantages: Memory resources are always occupied.

## Run method

### Prepare environment

First step, you could refer to [Run in TEE](./README.md#run-in-tee) for preparing environment.

### Run VLC TEE Images

```bash
cd image
cargo run --bin run-solo-llm-enclave -- . --features nitro-enclaves
```

## Testing

```bash
cargo run --bin call_llm_client --features nitro-enclaves -- 1
```

