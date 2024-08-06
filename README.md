# AOS TEE Operator

## Overview

The AOS TEE Operator is a role of [Aos-AVS](https://zsocial.gitbook.io/aos-network) in Hetu Protocols. 

By registering with AOS on Dispatcher, the operator could service the AI inference verification task.The staker can delegate funds to an operator by Delegation Manager contract.

The [Llm-In-TEE](#llm-in-tee) is a novelty framworks to run a TEE verification node service. And the AOS TEE Operators are TEE workers and building on Llm-In-TEE framwork.

## Llm-In-TEE

Run large AI models and verifiable logic clock in TEE environment.

Firstly, the semantic TEE of this repository is mainly refer to **aws nitro enclave** for now.  

Other TEE instances maybe support for later. For examples,
* Mircosoft Azure, 
* Intel SGX, 
* AMD SEV 
* or Nvidia Confidential Computing GPU

The Llm-In-TEE use the [llama.cpp](https://github.com/ggerganov/llama.cpp) as it's large AI models executor.

Second core module verifiable logic clock is an implementation of Chronos's TEE backend.   

The Chronos is a novel logical clock system designed for open networks with Byzantine participants, offering improved fault tolerance and performance. Please refer to [hetu chronos](https://github.com/hetu-project/chronos) repository for more details.

### Llm-In-TEE Arch

![architecture-diagram](./docs/img/architecture-diagram.png)

## Compile

### Build from source

```bash
git clone https://github.com/hetu-project/aos-tee-operator.git

cd llm-in-tee

git submodule update --init --recursive

cargo build --features nitro-enclaves --release
```

## Run TEE Operator

Now, this repository use the aws nitro enclave as its trust execution environment.  

So, please create a cloud virtual instance and notice choose the `Amazon-2023 linux` as base image.  
Because this base operator system is more friendly for using of the aws nitro enclave.

### Prepare Env & Configuration

1. Prepare Env & install dependency tools
```sh
sudo sudo dnf upgrade 
sudo dnf install -y tmux htop openssl-devel perl docker-24.0.5-1.amzn2023.0.3 aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel
``` 

2. Configuration

Please `cat /etc/nitro_enclaves/allocator.yaml` and set cpu_count & memory_mib. For tee_vlc: just `2 core + 1024 M` is enough, for tee_llm: `4 core + 16384 M` at least. Update the file and save it.

3. run `init.sh`

```sh
cd scripts
sudo chmod +x init_env.sh
./init_env.sh
```  
Remember please re-run the script when you update the `/etc/nitro_enclaves/allocator.yaml`.

### Run Operator

Please see [Run TEE Operator](./operator/README.md) for more detail information.