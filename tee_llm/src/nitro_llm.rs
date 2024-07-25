use common::{
    crypto::core::DigestHash,
    nitro_secure::{HandleFn, NitroSecureModule as NitroSecure},
    ordinary_clock::{Clock, LamportClock, OrdinaryClock},
    types::Payload,
};
use num_bigint::BigUint;
use vrf::{ecvrf::{Output, VRFPrivateKey, VRFPublicKey, OUTPUT_LENGTH}, sample::Sampler as VRFSampler};
use tools::helper::machine_used;
use anyhow::Ok;
use bincode::Options;
use rand::rngs::OsRng;
use std::{default, io};
use std::io::Write;
use std::{sync::Arc, time::Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::*;

use llama_cpp::standard_sampler::{SamplerStage, StandardSampler};
use llama_cpp::{LlamaModel, LlamaParams, SessionParams};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptReq {
    pub request_id: String,
    pub model_name: String,
    pub prompt: String,
    pub temperature: f32,
    pub top_p: f32,       // top p
    pub n_predict: usize, // maximum predict token
    pub vrf_prompt_hash: String,
    pub vrf_threshold: u64,
    pub vrf_precision: usize,
    // pub n_threads: u32,
    // pub clock: NitroEnclavesClock, // to be done
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnswerResp {
    pub request_id: String,
    pub model_name: String,
    pub prompt: String,
    pub answer: String,
    pub elapsed: u64,
    pub selected: bool,
    pub document: Payload,
    pub vrf_prompt_hash: String,
    pub vrf_random_value: String,
    pub vrf_verify_pubkey: String,
    pub vrf_proof: String,
    // pub clock: NitroEnclavesClock, // to be done
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct VRFReply {
    pub selected: bool,
    pub vrf_prompt_hash: String,
    pub vrf_random_value: String,
    pub vrf_verify_pubkey: String,
    pub vrf_proof: String,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct NitroEnclavesLlm {
    pub port: u32,
    pub cpu_core: u32,
    pub mem_mb: u32,
}

// technically `feature = "aws-nitro-enclaves-attestation"` is sufficient for
// attestation, NSM API is only depended by `NitroSecureModule` that running
// inside enclaves image
#[cfg(feature = "nitro-enclaves")]
impl AnswerResp {
    pub fn verify_inference(
        &self,
    ) -> anyhow::Result<Option<aws_nitro_enclaves_nsm_api::api::AttestationDoc>> {
        if self.answer.is_empty() {
            return Ok(None);
        }
        use aws_nitro_enclaves_attestation::{AttestationProcess as _, AWS_ROOT_CERT};
        use aws_nitro_enclaves_nsm_api::api::AttestationDoc;
        let document = AttestationDoc::from_bytes(
            &self.document,
            AWS_ROOT_CERT,
            std::time::SystemTime::UNIX_EPOCH
                .elapsed()
                .unwrap()
                .as_secs(),
        )?;
        use DigestHash as _;
        anyhow::ensure!(
            document.user_data.as_ref().map(|user_data| &***user_data)
                == Some(&self.answer.sha256().to_fixed_bytes()[..])
        );
        Ok(Some(document))
    }
}

#[cfg(feature = "nitro-enclaves")]
impl NitroEnclavesLlm {

    pub fn run_vrf(req: PromptReq) -> Result<VRFReply, anyhow::Error> {
        let private_key = VRFPrivateKey::generate_keypair(&mut OsRng);
        let public_key: VRFPublicKey = (&private_key).into();
        let proof: vrf::ecvrf::Proof = private_key.prove(req.vrf_prompt_hash.as_bytes());
        let output: Output = (&proof).into();
        let start = OUTPUT_LENGTH * 2 - req.vrf_precision;
        let end = OUTPUT_LENGTH * 2;
        let random_num = hex::encode(output.to_bytes());  
        let random_str = &random_num[start..end];
        let vrf_sampler = VRFSampler::new(req.vrf_precision * 4);
        let random_bigint = vrf_sampler.hex_to_biguint(random_str);
        let selected = vrf_sampler.meets_threshold(&random_bigint, &BigUint::from(req.vrf_threshold));
        Ok(VRFReply {
            selected,
            vrf_prompt_hash: req.vrf_prompt_hash,
            vrf_random_value: random_num,
            vrf_verify_pubkey: hex::encode(public_key.as_bytes()),
            vrf_proof: hex::encode(proof.to_bytes()),
        })
    }
    pub fn run_task(req: PromptReq) -> Result<String, anyhow::Error> {
        // llama format
        let params = LlamaParams::default();

        // Create a model from anything that implements `AsRef<Path>`:
        let model = LlamaModel::load_from_file(req.model_name.clone(), params)
            .expect("Could not load model");
        let cpu_nums = machine_used().1;
        let session_params = SessionParams {
            n_ctx: 4096,
            n_batch: 2048,
            n_ubatch: 512,
            n_threads: cpu_nums as u32,
            ..Default::default()
        };

        // A `LlamaModel` holds the weights shared across many _sessions_; while your model may be
        // several gigabytes large, a session is typically a few dozen to a hundred megabytes!
        let mut ctx = model
            .create_session(session_params)
            .expect("Failed to create session");

        // You can feed anything that implements `AsRef<[u8]>` into the model's context.
        ctx.advance_context(req.prompt).unwrap();

        // LLMs are typically used to predict the next word in a sequence. Let's generate some tokens!
        let mut decoded_tokens = 0;

        let sampler_stages = vec![
            SamplerStage::RepetitionPenalty {
                repetition_penalty: 1.0,
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
                last_n: 64,
            },
            SamplerStage::TopK(40),
            SamplerStage::TopP(req.top_p), // 0.95
            SamplerStage::MinP(0.05),
            SamplerStage::Typical(1.0),
            SamplerStage::Temperature(req.temperature),
        ];

        let sampler = StandardSampler::new_mirostat_v2(sampler_stages, 0, 0.1, 5.0);

        // `ctx.start_completing_with` creates a worker thread that generates tokens. When the completion
        // handle is dropped, tokens stop generating!
        let completions = ctx
            .start_completing_with(sampler, req.n_predict)?
            .into_strings();

        let mut answer = String::new();
        for completion in completions {
            answer.push_str(&completion);
            // print!("{completion}");
            // let _ = io::stdout().flush();

            decoded_tokens += 1;

            if decoded_tokens > req.n_predict {
                break;
            }
        }

        Ok(answer)
    }

    pub fn worker() -> HandleFn {
        Arc::new(|buf, nsm, pcrs, write_sender| {
            Box::pin(async move {
                if let Err(err) = async {
                    let req: PromptReq = bincode::options().deserialize::<PromptReq>(&buf)?;

                    anyhow::ensure!(true);
                    let mut answer = String::new();
                    let mut document = Vec::<u8>::new();
                    let start = Instant::now();
                    let vrf = NitroEnclavesLlm::run_vrf(req.clone())?;
                    if vrf.selected {
                        answer = NitroEnclavesLlm::run_task(req.clone())?;
                    }
                    let duration = start.elapsed();
                    // println!("\n\n Duration passed: {:?}", duration);
                    // let _ = io::stdout().flush();
                    
                    let user_data = answer.sha256().to_fixed_bytes().to_vec();
                    document = nsm.process_attestation(user_data)?;
                    let answer_doc = AnswerResp {
                        request_id: req.request_id,
                        model_name: req.model_name,
                        prompt: req.prompt.clone(),
                        answer,
                        elapsed: duration.as_secs(),
                        document: Payload(document),
                        selected: vrf.selected,
                        vrf_prompt_hash: vrf.vrf_prompt_hash,
                        vrf_random_value: vrf.vrf_random_value,
                        vrf_verify_pubkey: vrf.vrf_verify_pubkey,
                        vrf_proof: vrf.vrf_proof,
                    };

                    let buf = bincode::options().serialize(&answer_doc)?;
                    write_sender.send(buf)?;
                    Ok(())
                }
                .await
                {
                    warn!("{err}")
                }
                Ok(())
            })
        })
    }

    pub async fn run(port: u32) -> anyhow::Result<()> {
        let handler: HandleFn = NitroEnclavesLlm::worker();

        NitroSecure::run(port, handler).await
    }
}

pub async fn nitro_enclaves_portal_session(
    cid: u32,
    port: u32,
    mut events: UnboundedReceiver<PromptReq>,
    sender: UnboundedSender<AnswerResp>,
) -> anyhow::Result<()> {
    use std::os::fd::AsRawFd;

    use bincode::Options;
    use nix::sys::socket::{connect, socket, AddressFamily, SockFlag, SockType, VsockAddr};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let fd = socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )?;
    // this one is blocking, but should be instant, hopefully
    {
        let _span = tracing::debug_span!("connect").entered();
        connect(fd.as_raw_fd(), &VsockAddr::new(cid, port))?
    }
    let stream = std::os::unix::net::UnixStream::from(fd);
    stream.set_nonblocking(true)?;
    let stream = tokio::net::UnixStream::from_std(stream)?;
    let (mut read_half, mut write_half) = stream.into_split();
    let write_session = tokio::spawn(async move {
        while let Some(prompt) = events.recv().await {
            let buf = bincode::options().serialize(&prompt)?;
            write_half.write_u64_le(buf.len() as _).await?;
            write_half.write_all(&buf).await?;
        }
        anyhow::Ok(())
    });
    let read_session = tokio::spawn(async move {
        loop {
            let len = read_half.read_u64_le().await?;
            let mut buf = vec![0; len as _];
            read_half.read_exact(&mut buf).await?;
            sender.send(bincode::options().deserialize(&buf)?)?
        }
        #[allow(unreachable_code)] // for type hinting
        anyhow::Ok(())
    });
    tokio::select! {
        result = write_session => return result?,
        result = read_session => result??
    }
    anyhow::bail!("unreachable")
}

pub fn try_connection(cid: u32, port: u32) -> anyhow::Result<tokio::net::UnixStream> {
    use nix::sys::socket::{connect, socket, AddressFamily, SockFlag, SockType, VsockAddr};
    use std::os::fd::AsRawFd;

    let fd = socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )?;

    {
        let _span = tracing::debug_span!("connect").entered();
        connect(fd.as_raw_fd(), &VsockAddr::new(cid, port))?
    }

    let stream = std::os::unix::net::UnixStream::from(fd);
    stream.set_nonblocking(true)?;

    let stream = tokio::net::UnixStream::from_std(stream)?;
    Ok(stream)
}

pub async fn tee_start_listening(
    stream: tokio::net::UnixStream,
    mut events: UnboundedReceiver<PromptReq>,
    sender: UnboundedSender<AnswerResp>,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let (mut read_half, mut write_half) = stream.into_split();

    let write_session = tokio::spawn(async move {
        while let Some(prompt) = events.recv().await {
            let buf = bincode::options().serialize(&prompt)?;
            write_half.write_u64_le(buf.len() as _).await?;
            write_half.write_all(&buf).await?;
        }
        anyhow::Ok(())
    });

    let read_session = tokio::spawn(async move {
        loop {
            let len = read_half.read_u64_le().await?;
            let mut buf = vec![0; len as _];
            read_half.read_exact(&mut buf).await?;
            sender.send(bincode::options().deserialize(&buf)?)?
        }
        #[allow(unreachable_code)] // for type hinting
        anyhow::Ok(())
    });

    tokio::select! {
        result = write_session => return result?,
        result = read_session => result??
    }

    anyhow::bail!("unreachable")
}
