use common::{
    crypto::DigestHash,
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
}

#[derive(Debug)]
pub struct NitroSecureModule(pub i32);

#[cfg(feature = "nitro-enclaves")]
impl NitroSecureModule {
    fn new() -> anyhow::Result<Self> {
        let fd = aws_nitro_enclaves_nsm_api::driver::nsm_init();
        anyhow::ensure!(fd >= 0);
        Ok(Self(fd))
    }

    fn process_attestation(&self, user_data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        use aws_nitro_enclaves_nsm_api::api::Request::Attestation;
        // some silly code to avoid explicitly mention `serde_bytes::ByteBuf`
        let mut request = Attestation {
            user_data: Some(Default::default()),
            nonce: None,
            public_key: None,
        };
        let Attestation {
            user_data: Some(buf),
            ..
        } = &mut request
        else {
            unreachable!()
        };
        buf.extend(user_data);
        match aws_nitro_enclaves_nsm_api::driver::nsm_process_request(self.0, request) {
            aws_nitro_enclaves_nsm_api::api::Response::Attestation { document } => Ok(document),
            aws_nitro_enclaves_nsm_api::api::Response::Error(err) => anyhow::bail!("{err:?}"),
            _ => anyhow::bail!("unimplemented"),
        }
    }

    fn describe_pcr(&self, index: u16) -> anyhow::Result<Vec<u8>> {
        use aws_nitro_enclaves_nsm_api::api::Request::DescribePCR;
        match aws_nitro_enclaves_nsm_api::driver::nsm_process_request(self.0, DescribePCR { index })
        {
            aws_nitro_enclaves_nsm_api::api::Response::DescribePCR { lock: _, data } => Ok(data),
            aws_nitro_enclaves_nsm_api::api::Response::Error(err) => anyhow::bail!("{err:?}"),
            _ => anyhow::bail!("unimplemented"),
        }
    }

    pub async fn run() -> anyhow::Result<()> {
        use std::os::fd::AsRawFd;

        use bincode::Options;
        use nix::sys::socket::{
            bind, listen, socket, AddressFamily, Backlog, SockFlag, SockType, VsockAddr,
        };
        use tokio::{
            io::{AsyncReadExt as _, AsyncWriteExt as _},
            sync::mpsc::unbounded_channel,
        };
        use DigestHash as _;

        let nsm = std::sync::Arc::new(Self::new()?);
        let pcrs = [
            nsm.describe_pcr(0)?,
            nsm.describe_pcr(1)?,
            nsm.describe_pcr(2)?,
        ];

        let socket_fd = socket(
            AddressFamily::Vsock,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )?;
        bind(socket_fd.as_raw_fd(), &VsockAddr::new(0xFFFFFFFF, 5005))?;
        // theoretically this is the earliest point to entering Tokio world, but i don't want to go
        // unsafe with `FromRawFd`, and Tokio don't have a `From<OwnedFd>` yet
        listen(&socket_fd, Backlog::new(64)?)?;
        let socket = std::os::unix::net::UnixListener::from(socket_fd);
        socket.set_nonblocking(true)?;
        let socket = tokio::net::UnixListener::from_std(socket)?;

        loop {
            let (stream, _) = socket.accept().await?;
            let (mut read_half, mut write_half) = stream.into_split();
            let (write_sender, mut write_receiver) = unbounded_channel::<Vec<_>>();

            let mut write_session = tokio::spawn(async move {
                while let Some(buf) = write_receiver.recv().await {
                    write_half.write_u64_le(buf.len() as _).await?;
                    write_half.write_all(&buf).await?;
                }
                anyhow::Ok(())
            });
            let nsm = nsm.clone();
            let pcrs = pcrs.clone();
            let mut read_session = tokio::spawn(async move {
                loop {
                    let task = async {
                        let len = read_half.read_u64_le().await?;
                        let mut buf = vec![0; len as _];
                        read_half.read_exact(&mut buf).await?;
                        anyhow::Ok(buf)
                    };
                    let buf = match task.await {
                        Ok(buf) => buf,
                        Err(err) => {
                            warn!("{err}");
                            return anyhow::Ok(());
                        }
                    };
                    let nsm = nsm.clone();
                    let pcrs = pcrs.clone();
                    let write_sender = write_sender.clone();
                    tokio::spawn(async move {
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
                    });
                }
            });
            loop {
                let result = tokio::select! {
                    result = &mut read_session, if !read_session.is_finished() => result,
                    result = &mut write_session, if !write_session.is_finished() => result,
                    else => break,
                };
                if let Err(err) = result.map_err(Into::into).and_then(std::convert::identity) {
                    warn!("{err}")
                }
            }
        }
    }
}

#[cfg(feature = "nitro-enclaves")]
impl Drop for NitroSecureModule {
    fn drop(&mut self) {
        aws_nitro_enclaves_nsm_api::driver::nsm_exit(self.0)
    }
}

pub async fn nitro_enclaves_portal_session(
    cid: u32,
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
        connect(fd.as_raw_fd(), &VsockAddr::new(cid, 5005))?
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

    use crate::{Clocked, Verify};
    use super::NitroEnclavesClock;

    impl<M: Send + Sync + 'static> Verify<()> for Clocked<M, NitroEnclavesClock> {
        fn verify_clock(&self, _: usize, (): &()) -> anyhow::Result<()> {
            self.clock.verify()?;
            Ok(())
        }
    }
}

