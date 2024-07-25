use std::{env, fmt::Write, future::pending, time::Duration};

use tee_llm::nitro_llm::{nitro_enclaves_portal_session, AnswerResp, PromptReq};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::{sleep, Instant},
};

// tee id
const CID: u32 = 15;

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
        let (update_ok_sender, mut update_ok_receiver) = unbounded_channel::<AnswerResp>();
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
                5005,
                update_receiver,
                update_ok_sender,
            )),
            tokio::spawn(async move {
                let verify = |answer: AnswerResp| {
                    let document = answer.verify_inference()?;
                    anyhow::ensure!(document.is_some());
                    Ok(())
                };

                let mut lines = String::new();
                if let Some(num_concurrent) = num_concurrent {
                    bench_session(
                        num_concurrent,
                        &update_sender,
                        &mut update_ok_receiver,
                        verify,
                        &mut lines,
                    )
                    .await?;
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

async fn bench_session(
    count: usize,
    update_sender: &UnboundedSender<PromptReq>,
    update_ok_receiver: &mut UnboundedReceiver<AnswerResp>,
    verify: impl Fn(AnswerResp) -> anyhow::Result<()>,
    lines: &mut String,
) -> anyhow::Result<()> {
    // fixed args for testing
    let req = PromptReq {
        request_id: "todo!()".to_owned(),
        model_name: "./llama-2-7b-chat.Q4_0.gguf".to_owned(),
        prompt: "How to combine AI and blockchain?".to_owned(),
        top_p: 0.95,
        temperature: 0.0,
        n_predict: 128,
        vrf_threshold: 16777215,
        vrf_precision: 6,
        vrf_prompt_hash: "sfas".to_owned(),
    };

    for _ in 0..count {
        sleep(Duration::from_millis(100)).await;
        let start = Instant::now();
        update_sender.send(req.clone())?;
        let Some(answer) = update_ok_receiver.recv().await else {
            anyhow::bail!("missing UpdateOk")
        };
        let elapsed = start.elapsed();
        eprintln!("outer elapsed: {elapsed:?}");
        writeln!(
            lines,
            "{count} times outer elapsed: {}",
            elapsed.as_secs_f32()
        )?;
        println!("answer: {:?}", answer);
        verify(answer)?
    }
    Ok(())
}
