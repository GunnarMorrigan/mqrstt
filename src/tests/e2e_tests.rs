#[cfg(test)]
mod e2e {

    use futures_concurrency::future::Join;

    use tracing::{Level};
    use tracing_subscriber::FmtSubscriber;

    use crate::{
        connect_options::ConnectOptions, create_new_tcp, error::ClientError, packets::{QoS, packets::{Packet, PacketType}}, event_handler::EventHandler, client::AsyncClient,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pub_qo_s2() {

        pub struct TestPubQoS2 {
            stage: StagePubQoS2,
            client: AsyncClient
        }
        pub enum StagePubQoS2{
            ConnAck,
            PubRec,
            PubComp,
            Done,
        }
        impl TestPubQoS2{
            fn new(client: AsyncClient) -> Self{
                TestPubQoS2 {
                    stage: StagePubQoS2::ConnAck,
                    client,
                }
            }
        }
        impl EventHandler for TestPubQoS2 {
            fn handle<'a>(
                &'a mut self,
                event: &'a Packet,
            ) -> impl core::future::Future<Output = ()> + Send + 'a {
                async move {
                    match self.stage{
                        StagePubQoS2::ConnAck => {
                            assert_eq!(event.packet_type(), PacketType::ConnAck);
                            self.stage = StagePubQoS2::PubRec;
                        },
                        StagePubQoS2::PubRec => {
                            assert_eq!(event.packet_type(), PacketType::PubRec);
                            self.stage = StagePubQoS2::PubComp;
                        },
                        StagePubQoS2::PubComp => {
                                assert_eq!(event.packet_type(), PacketType::PubComp);
                                self.stage = StagePubQoS2::Done;
                                self.client.disconnect().await.unwrap();
                        },
                        StagePubQoS2::Done => {
                            ()
                        },
                    }
                }
            }
        }

        let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_max_level(Level::TRACE)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        // let opt = ConnectOptions::new("broker.emqx.io".to_string(), 1883, "test123123".to_string());
        let opt = ConnectOptions::new(
            "127.0.0.1".to_string(),
            1883,
            "test123123".to_string(),
        );

        let (mut mqtt_network, handler, client, _r) = create_new_tcp(opt);

        let network =tokio::task::spawn(async move { 
            dbg!(mqtt_network.run().await) 
        });

        let client_cloned = client.clone();
        let event_handler = tokio::task::spawn(async move {
            let mut custom_handler = TestPubQoS2::new(client_cloned);
            dbg!(handler.handle(&mut custom_handler).await)
        });

        let sender = tokio::task::spawn(async move {
            client
                .publish(QoS::ExactlyOnce, false, "test/123".to_string(), "123456789")
                .await?;

            let lol = smol::future::pending::<Result<(), ClientError>>();
            lol.await
        });

        dbg!((network, event_handler, sender).join().await);
    }
}
