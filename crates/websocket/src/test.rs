/// zchronod_websocket test

use crate::*;

#[tokio::test(flavor = "multi_thread")]
async fn test1() {

    #[derive(Debug, serde::Serialize, serde::Deserialize,PartialEq)]
    enum TestMsg {
        HI,
    }

    let (addr_s, addr_r) = tokio::sync::oneshot::channel();

    let l_task = tokio::task::spawn(async move {
        let l = WebsocketListener::bind(Arc::new(WebsocketConfig::default()), "localhost:0")
            .await
            .unwrap();

        let addr = l.local_addr().unwrap();
        addr_s.send(addr).unwrap();

        let (_send, mut recv) = l.accept().await.unwrap();


        let res = recv.recv().await.unwrap();
        match res {
            ReceiveMessage::Request(data, res) => {
                assert_eq!(serde_json::to_vec(&TestMsg::HI).unwrap(), data);
                res.respond(serde_json::to_vec(&TestMsg::HI).unwrap()).await.unwrap();
            }
            oth => panic!("unexpected: {oth:?}"),
        }
    });

    let addr = addr_r.await.unwrap();
    println!("addr: {}", addr);

    let r_task = tokio::task::spawn(async move {
        let (send, mut recv) = connect(Arc::new(WebsocketConfig::default()), addr)
            .await
            .unwrap();


        let s_task =
            tokio::task::spawn(async move { while let Ok(_r) = recv.recv().await {} });

        let res = send
            .request_timeout(serde_json::to_vec(&TestMsg::HI).unwrap(), std::time::Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(TestMsg::HI, serde_json::from_slice(res.as_slice()).unwrap());

        s_task.abort();
    });

    l_task.await.unwrap();
    r_task.await.unwrap();
}
