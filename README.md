# Llm-In-TEE

## Overview

Run large AI models and verifiable logic clock in TEE environment.

Firstly, the semantic TEE of this repository is mainly refer to **aws nitro enclave** for now.  

Other TEE instances maybe support for later. For examples,
* Mircosoft Azure, 
* Intel SGX, 
* AMD SEV 
* or Nvidia Confidential Computing GPU

Second core module verifiable logic clock is an implementation of Chronos's TEE backend.   

The Chronos is a novel logical clock system designed for open networks with Byzantine participants, offering improved fault tolerance and performance. Please refer to [hetu chronos](https://github.com/hetu-project/chronos) repository for more details.

## Architecture

![architecture-diagram](./docs/img/architecture-diagram.png)

## Compile

### Build from source

```bash
git clone https://github.com/ai-chen2050/llm-in-tee.git

cd llm-in-tee

cargo build
```