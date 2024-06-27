use std::{
    env,
    fmt::Write,
    future::pending,
    // net::{IpAddr, SocketAddr},
    time::Duration,
};

use common::ordinary_clock::OrdinaryClock;
use tee_vlc::nitro_clock::{nitro_enclaves_portal_session, NitroEnclavesClock, Update, UpdateOk};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::{sleep, timeout, Instant},
};

// tee id
const CID: u32 = 16;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    let num_concurrent = if args.len() > 1 {
        args[1].parse::<usize>().ok()
    } else {
        None
    };

    let run_nitro_client = {
        let (update_sender, update_receiver) = unbounded_channel();
        let (update_ok_sender, mut update_ok_receiver) = unbounded_channel::<UpdateOk<_>>();
        tokio::spawn({
            let update_sender = update_sender.clone();
            async move {
                pending::<()>().await;
                drop(update_sender)
            }
        });
        (
            tokio::spawn(nitro_enclaves_portal_session(
                CID,
                update_receiver,
                update_ok_sender,
            )),
            tokio::spawn(async move {
                let verify = |clock: NitroEnclavesClock| {
                    let document = clock.verify()?;
                    anyhow::ensure!(document.is_some());
                    Ok(())
                };
                let mut lines = String::new();
                if let Some(num_concurrent) = num_concurrent {
                    stress_bench_session(
                        1 << 10,
                        0,
                        num_concurrent,
                        &update_sender,
                        &mut update_ok_receiver,
                        &mut lines,
                    )
                    .await?;
                    println!("{lines}")
                } else {
                    for size in (0..=16).step_by(2).map(|n| 1 << n) {
                        bench_session(
                            size,
                            0,
                            &update_sender,
                            &mut update_ok_receiver,
                            verify,
                            &mut lines,
                        )
                        .await?
                    }
                    for num_merged in 0..=15 {
                        bench_session(
                            1 << 10,
                            num_merged,
                            &update_sender,
                            &mut update_ok_receiver,
                            verify,
                            &mut lines,
                        )
                        .await?
                    }
                    println!("{lines}")
                }

                anyhow::Ok(())
            }),
        )
    };

    let (portal_session, session) = run_nitro_client;
    'select: {
        tokio::select! {
            result = session => break 'select result??,
            result = portal_session => result??,
        }
        anyhow::bail!("unreachable")
    }
    Ok(())
}

async fn bench_session<C: TryFrom<OrdinaryClock> + Clone + Send + Sync + 'static>(
    size: usize,
    num_merged: usize,
    update_sender: &UnboundedSender<Update<C>>,
    update_ok_receiver: &mut UnboundedReceiver<UpdateOk<C>>,
    verify: impl Fn(C) -> anyhow::Result<()>,
    lines: &mut String,
) -> anyhow::Result<()>
where
    C::Error: Into<anyhow::Error>,
{
    let clock =
        C::try_from(OrdinaryClock((0..size).map(|i| (i as _, 0)).collect())).map_err(Into::into)?;
    update_sender.send(Update(clock, Default::default(), 0))?;
    let Some((_, clock)) = update_ok_receiver.recv().await else {
        anyhow::bail!("missing UpdateOk")
    };
    for _ in 0..10 {
        sleep(Duration::from_millis(100)).await;
        let update = Update(clock.clone(), vec![clock.clone(); num_merged], 0);
        let start = Instant::now();
        update_sender.send(update)?;
        let Some((_, clock)) = update_ok_receiver.recv().await else {
            anyhow::bail!("missing UpdateOk")
        };
        let elapsed = start.elapsed();
        eprintln!("{size:8} {num_merged:3} {elapsed:?}");
        writeln!(lines, "{size},{num_merged},{}", elapsed.as_secs_f32())?;
        verify(clock)?
    }
    Ok(())
}

async fn stress_bench_session<C: TryFrom<OrdinaryClock> + Clone + Send + Sync + 'static>(
    size: usize,
    num_merged: usize,
    num_concurrent: usize,
    update_sender: &UnboundedSender<Update<C>>,
    update_ok_receiver: &mut UnboundedReceiver<UpdateOk<C>>,
    lines: &mut String,
) -> anyhow::Result<()>
where
    C::Error: Into<anyhow::Error>,
{
    let clock =
        C::try_from(OrdinaryClock((0..size).map(|i| (i as _, 0)).collect())).map_err(Into::into)?;
    for i in 0..num_concurrent {
        update_sender.send(Update(clock.clone(), Default::default(), i as _))?;
    }
    let mut count = 0;
    let close_loops_session = async {
        while let Some((id, clock)) = update_ok_receiver.recv().await {
            count += 1;
            let update = Update(clock.clone(), vec![clock.clone(); num_merged], id);
            update_sender.send(update)?
        }
        anyhow::Ok(())
    };
    match timeout(Duration::from_secs(10), close_loops_session).await {
        Err(_) => {}
        Ok(result) => {
            result?;
            anyhow::bail!("unreachable")
        }
    }
    eprintln!("concurrent {num_concurrent} count {count}");
    writeln!(
        lines,
        "{size},{num_merged},{num_concurrent},{}",
        count as f32 / 10.
    )?;
    Ok(())
}
