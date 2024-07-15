use std::sync::Arc;
use bincode::Options;
// use std::io;
// use std::io::Write;

use common::{
    crypto::core::DigestHash,
    nitro_secure::{HandleFn, NitroSecureModule as NitroSecure},
    ordinary_clock::{Clock, LamportClock, OrdinaryClock},
    types::Payload,
};
use derive_where::derive_where;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Update<C>(pub C, pub Vec<C>, pub u64);

// feel lazy to define event type for replying
pub type UpdateOk<C> = (u64, C);

#[derive(Debug, Clone, Hash, derive_more::AsRef, Serialize, Deserialize)]
#[derive_where(PartialOrd, PartialEq)]
pub struct NitroEnclavesClock {
    #[as_ref]
    pub plain: OrdinaryClock,
    #[derive_where(skip)]
    pub document: Payload,
}

impl TryFrom<OrdinaryClock> for NitroEnclavesClock {
    type Error = anyhow::Error;

    fn try_from(value: OrdinaryClock) -> Result<Self, Self::Error> {
        anyhow::ensure!(value.is_genesis());
        Ok(Self {
            plain: value,
            document: Default::default(),
        })
    }
}

impl Clock for NitroEnclavesClock {
    fn reduce(&self) -> LamportClock {
        self.plain.reduce()
    }
}

// technically `feature = "aws-nitro-enclaves-attestation"` is sufficient for
// attestation, NSM API is only depended by `NitroSecureModule` that running
// inside enclaves image
#[cfg(feature = "nitro-enclaves")]
impl NitroEnclavesClock {
    pub fn verify(
        &self,
    ) -> anyhow::Result<Option<aws_nitro_enclaves_nsm_api::api::AttestationDoc>> {
        if self.plain.is_genesis() {
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
                == Some(&self.plain.sha256().to_fixed_bytes()[..])
        );
        Ok(Some(document))
    }

    pub fn worker() -> HandleFn {
        Arc::new(|buf, nsm, pcrs, write_sender| {
            Box::pin(async move {
                // IO action in tee is severe delay, just debug
                // println!("Received buffer: {:?}", buf);
                // let _ = io::stdout().flush();
    
                if let Err(err) = async {
                    let Update(prev, merged, id) = bincode::options()
                        .deserialize::<Update<NitroEnclavesClock>>(&buf)?;
                    for clock in [&prev].into_iter().chain(&merged) {
                        if let Some(document) = clock.verify()? {
                            for (i, pcr) in pcrs.iter().enumerate() {
                                anyhow::ensure!(
                                    document.pcrs.get(&i).map(|pcr| &**pcr) == Some(pcr)
                                )
                            }
                        }
                    }
                    let plain = prev
                        .plain
                        .update(merged.iter().map(|clock| &clock.plain), id);
                    
                    // relies on the fact that different clocks always hash into different
                    // digests, hopefully true
                    let user_data = plain.sha256().to_fixed_bytes().to_vec();
                    let document = nsm.process_attestation(user_data)?;
                    let updated = NitroEnclavesClock {
                        plain,
                        document: Payload(document),
                    };
                    let buf = bincode::options().serialize(&(id, updated))?;
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
        let handler: HandleFn = NitroEnclavesClock::worker();

        NitroSecure::run(port, handler).await
    }
}


pub async fn nitro_enclaves_portal_session(
    cid: u32,
    port: u32,
    mut events: UnboundedReceiver<Update<NitroEnclavesClock>>,
    sender: UnboundedSender<UpdateOk<NitroEnclavesClock>>,
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
        while let Some(update) = events.recv().await {
            let buf = bincode::options().serialize(&update)?;
            write_half.write_u64_le(buf.len() as _).await?;
            write_half.write_all(&buf).await?
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

#[cfg(feature = "nitro-enclaves")]
pub mod impls {

    use super::NitroEnclavesClock;
    use crate::{Clocked, Verify};

    impl<M: Send + Sync + 'static> Verify<()> for Clocked<M, NitroEnclavesClock> {
        fn verify_clock(&self, _: usize, (): &()) -> anyhow::Result<()> {
            self.clock.verify()?;
            Ok(())
        }
    }
}
