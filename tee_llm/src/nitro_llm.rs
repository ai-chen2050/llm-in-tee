use bincode::Options;
use std::{sync::Arc, time::Instant};
use std::io;
use std::io::Write;

use common::{
    crypto::DigestHash,
    nitro_secure::{HandleFn, NitroSecureModule as NitroSecure},
    ordinary_clock::{Clock, LamportClock, OrdinaryClock},
    types::Payload,
};
use serde::{Deserialize, Serialize};
use tracing::*;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use llama_cpp::standard_sampler::{SamplerStage, StandardSampler};
use llama_cpp::{
    LlamaModel, LlamaParams, SessionParams
};

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct PromptReq {
    pub model_name: String,
    pub prompt: String,
    pub n_ctx: u32, // contex maximum token
    pub n_predict: usize, // maximum predict token
    pub n_threads: u32,
    // pub clock: NitroEnclavesClock, // to be done
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::AsRef)]
pub struct AnswerResp {
    pub answer: String,
    pub elapsed: u64,
    pub document: Payload,
    // pub clock: NitroEnclavesClock, // to be done
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
    pub fn verify(
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
    pub fn run_task(req: PromptReq) -> Result<String, anyhow::Error> {
        // llama format
        let params = LlamaParams::default();

        // Create a model from anything that implements `AsRef<Path>`:
        let model = LlamaModel::load_from_file(
            req.model_name
                .clone(),
                params,
        )
        .expect("Could not load model");

        let session_params = SessionParams {
            n_ctx: req.n_ctx,
            n_batch: 2048,
            n_ubatch: 512,
            n_threads: req.n_threads,
            ..Default::default()
        };

        // A `LlamaModel` holds the weights shared across many _sessions_; while your model may be
        // several gigabytes large, a session is typically a few dozen to a hundred megabytes!
        let mut ctx = model
            .create_session(session_params)
            .expect("Failed to create session");

        // You can feed anything that implements `AsRef<[u8]>` into the model's context.
        ctx.advance_context(req.prompt)
            .unwrap();

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
            SamplerStage::TopP(0.95),
            SamplerStage::MinP(0.05),
            SamplerStage::Typical(1.0),
            SamplerStage::Temperature(0.0),
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
                    let req: PromptReq = bincode::options()
                        .deserialize::<PromptReq>(&buf)?;
                    
                    anyhow::ensure!(true);
                    let start = Instant::now();
                    let answer = NitroEnclavesLlm::run_task(req)?;
                    let duration = start.elapsed();
                    
                    // println!("\n\n Duration passed: {:?}", duration);
                    // let _ = io::stdout().flush();
                    
                    let user_data = answer.sha256().to_fixed_bytes().to_vec();
                    let document = nsm.process_attestation(user_data)?;
                    let answer_doc = AnswerResp {
                        answer,
                        elapsed: duration.as_secs(),
                        document: Payload(document),
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
    use std::os::fd::AsRawFd;
    use nix::sys::socket::{connect, socket, AddressFamily, SockFlag, SockType, VsockAddr};

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

pub async fn start_listening(
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