#[cfg(feature = "tokio")]
#[cfg(test)]
mod tokio_e2e {

    use futures_concurrency::future::Join;

    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    use crate::tests::stages::Nop;
    use crate::{
        connect_options::ConnectOptions, create_tokio_tcp, error::ClientError,
        event_handler::EventHandler, packets::QoS,
    };

    use crate::tests::stages::qos_2::TestPubQoS2;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pub_qos_2() {
        let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_max_level(Level::TRACE)
            .with_line_number(true)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        let opt = ConnectOptions::new_with_tls_config(
            "broker.emqx.io".to_string(),
            1883,
            "test123123".to_string(),
            None,
        );
        // let opt = ConnectOptions::new("127.0.0.1".to_string(), 1883, "test123123".to_string(), None);

        let (mut mqtt_network, handler, client) = create_tokio_tcp(opt);

        let network = tokio::task::spawn(async move { dbg!(mqtt_network.run().await) });

        let event_handler = tokio::task::spawn(async move {
            let mut custom_handler = Nop{};
            dbg!(handler.handle(&mut custom_handler).await)
        });

        let sender = tokio::task::spawn(async move {
            client.subscribe("mqrstt").await.unwrap();
            client
                .publish(QoS::ExactlyOnce, false, "test".to_string(), "123456789")
                .await?;

            let lol = smol::future::pending::<Result<(), ClientError>>();
            lol.await
        });

        dbg!((network, event_handler, sender).join().await);
    }
}

// #[cfg(all(feature = "smol", feature = "smol-rustls"))]
// #[cfg(test)]
// mod smol_rustls_e2e {

//     use tracing::Level;
//     use tracing_subscriber::FmtSubscriber;

//     use crate::{
//         connect_options::ConnectOptions, connections::transport::RustlsConfig, create_smol_rustls,
//         tests::resources::EMQX_CERT,
//     };

//     #[test]
//     fn test_pub_tcp_qos_2() {
//         let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

//         let subscriber = FmtSubscriber::builder()
//             .with_env_filter(filter)
//             .with_max_level(Level::TRACE)
//             .with_line_number(true)
//             .finish();

//         tracing::subscriber::set_global_default(subscriber)
//             .expect("setting default subscriber failed");

//         let config = RustlsConfig::Simple {
//             ca: EMQX_CERT.to_vec(),
//             alpn: None,
//             client_auth: None,
//         };

//         let opt = ConnectOptions::new("broker.emqx.io".to_string(), 8883, "test123123".to_string());

//         let (mut mqtt_network, _handler, _client) = create_smol_rustls(opt, config);

//         smol::block_on(mqtt_network.run()).unwrap()
//     }
// }
