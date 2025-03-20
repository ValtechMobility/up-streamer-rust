mod common;

use common::{start_streamer, MessageCollector, CLIENT_URI, SERVICE_URI};
use hello_world_protos::hello_world_service::HelloRequest;
use std::{str::FromStr, sync::Arc};
use up_rust::{UMessageBuilder, UStatus, UTransport, UUri};
use up_transport_mqtt5::{Mqtt5Transport, MqttClientOptions, TransportMode};
use up_transport_zenoh::UPTransportZenoh;
use zenoh::config::{Config, EndPoint};

#[tokio::test]
async fn test_mqtt_client_to_zenoh_service() {
    start_streamer("mqtt_client_zenoh_service_config.json5")
        .await
        .unwrap();

    let client_collector = Arc::new(MessageCollector::new(1));
    let service_collector = Arc::new(MessageCollector::new(1));

    let mqtt_client = setup_mqtt_client(client_collector.clone()).await.unwrap();
    let _zenoh_service = setup_zenoh_service(service_collector.clone());

    let request = HelloRequest {
        name: "test_request".to_string(),
        ..Default::default()
    };

    send_request_and_wait_for_response(mqtt_client, request).await;

    let messages = service_collector.get_messages().await;
    assert_eq!(messages.len(), 1);
    let request_id = messages[0].request_id().unwrap();

    let messages = client_collector.get_messages().await;
    assert_eq!(messages.len(), 1);

    let response_id = messages[0].request_id().unwrap();

    assert_eq!(request_id, response_id)
}

async fn setup_mqtt_client(
    collector: Arc<MessageCollector>,
) -> Result<Arc<dyn UTransport>, UStatus> {
    let service_uri = UUri::try_from(SERVICE_URI).unwrap();
    let client_uri = UUri::try_from(CLIENT_URI).unwrap();

    let mqtt_options = MqttClientOptions {
        broker_uri: "localhost:1883".to_string(),
        ..Default::default()
    };

    let mqtt5_transport =
        Mqtt5Transport::new(TransportMode::InVehicle, mqtt_options, "client".to_string()).await?;
    mqtt5_transport.connect().await?;

    mqtt5_transport
        .register_listener(&client_uri, Some(&service_uri), collector.clone())
        .await
        .unwrap();

    Ok(Arc::new(mqtt5_transport))
}

async fn setup_zenoh_service(
    collector: Arc<MessageCollector>,
) -> Result<Arc<dyn UTransport>, UStatus> {
    let service_uri = UUri::try_from(SERVICE_URI).unwrap();

    let mut zenoh_options = Config::default();
    let endpoint = EndPoint::from_str("tcp/localhost:7447").unwrap();

    zenoh_options.connect.endpoints.set(vec![endpoint]).unwrap();

    let zenoh_transport: Arc<dyn UTransport> = Arc::new(
        UPTransportZenoh::new(zenoh_options, service_uri.clone())
            .await
            .unwrap(),
    );

    zenoh_transport
        .register_listener(&UUri::any(), Some(&service_uri), collector.clone())
        .await
        .unwrap();

    Ok(zenoh_transport)
}

async fn send_request_and_wait_for_response(client: Arc<dyn UTransport>, request: HelloRequest) {
    let source_uri = UUri::try_from(CLIENT_URI).unwrap();
    let sink_uri = UUri::try_from(SERVICE_URI).unwrap();
    let message = UMessageBuilder::request(sink_uri, source_uri, 5)
        .build_with_protobuf_payload(&request)
        .unwrap();

    client.send(message).await.unwrap();
}
