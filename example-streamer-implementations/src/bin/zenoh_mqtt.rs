/********************************************************************************
 * Copyright (c) 2024 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

mod config;

use crate::config::Config;
use clap::Parser;
use log::{info, trace};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::thread;
use up_rust::{UCode, UStatus, UTransport, UUri, UUID};
use up_streamer::{Endpoint, UStreamer};
use up_transport_mqtt5::{Mqtt5Transport, MqttClientOptions, TransportMode};
use up_transport_zenoh::UPTransportZenoh;
use usubscription_static_file::USubscriptionStaticFile;
use zenoh::config::Config as ZenohConfig;

#[derive(Parser)]
#[command()]
struct StreamerArgs {
    #[arg(short, long, value_name = "FILE")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), UStatus> {
    env_logger::init();

    info!("Started up-linux-streamer-mqtt-zenoh");

    // Get the config file.
    let args = StreamerArgs::parse();
    let mut file = File::open(args.config)
        .map_err(|e| UStatus::fail_with_code(UCode::NOT_FOUND, format!("File not found: {e:?}")))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| {
        UStatus::fail_with_code(
            UCode::INTERNAL,
            format!("Unable to read config file: {e:?}"),
        )
    })?;

    let config: Config = json5::from_str(&contents).map_err(|e| {
        UStatus::fail_with_code(
            UCode::INTERNAL,
            format!("Unable to parse config file: {e:?}"),
        )
    })?;

    let subscription_path = config.usubscription_config.file_path;
    let usubscription = Arc::new(USubscriptionStaticFile::new(subscription_path));

    // Start the streamer instance.
    let mut streamer = UStreamer::new(
        "up-streamer",
        config.up_streamer_config.message_queue_size,
        usubscription,
    )
    .expect("Failed to create uStreamer");

    // In this implementation we define that the streamer lives in the same ecu component as the zenoh entity and so shares its authority name but with a different service ID.
    let streamer_uuri = UUri::try_from_parts(
        &config.streamer_uuri.authority,
        config.streamer_uuri.ue_id,
        config.streamer_uuri.ue_version_major,
        0,
    )
    .expect("Unable to form streamer_uuri");

    trace!("streamer_uuri: {streamer_uuri:#?}");
    let zenoh_config = ZenohConfig::from_file(config.zenoh_transport_config.config_file).unwrap();

    let zenoh_transport: Arc<dyn UTransport> = Arc::new(
        UPTransportZenoh::new(zenoh_config, streamer_uuri)
            .await
            .expect("Unable to initialize Zenoh UTransport"),
    );

    // Because the streamer runs on the ecu side in this implementation, we call the zenoh endpoint the "host".
    let zenoh_endpoint = Endpoint::new(
        "host_endpoint",
        &config.streamer_uuri.authority,
        zenoh_transport.clone(),
    );

    let mut mqtt_options = MqttClientOptions::default();
    mqtt_options.broker_uri =
        config.mqtt_config.hostname + ":" + &config.mqtt_config.port.to_string();

    let mqtt5_transport = Mqtt5Transport::new(
        TransportMode::InVehicle,
        mqtt_options,
        config.mqtt_config.authority.clone(),
    )
    .await?;
    mqtt5_transport.connect().await?;

    let mqtt5_transport: Arc<dyn UTransport> = Arc::new(mqtt5_transport);

    // In this implementation, the mqtt entity runs in the cloud and has its own authority.
    let mqtt_endpoint = Endpoint::new(
        "cloud_endpoint",
        &config.mqtt_config.authority,
        mqtt5_transport.clone(),
    );

    // Here we tell the streamer to forward any zenoh messages to the mqtt endpoint
    streamer
        .add_forwarding_rule(zenoh_endpoint.clone(), mqtt_endpoint.clone())
        .await
        .expect("Could not add zenoh -> mqtt forwarding rule");

    // And here we set up the forwarding in the other direction.
    streamer
        .add_forwarding_rule(mqtt_endpoint.clone(), zenoh_endpoint.clone())
        .await
        .expect("Could not add mqtt -> zenoh forwarding rule");

    thread::park();

    Ok(())
}
