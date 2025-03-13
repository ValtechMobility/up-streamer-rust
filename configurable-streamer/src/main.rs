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
use log::info;
use std::io::Read;
use std::sync::Arc;
use std::thread;
use std::{collections::HashMap, fs::File};
use up_rust::{UCode, UStatus, UTransport, UUri};
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

    info!("Started up-linux-streamer-configurable");

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

    let mut config: Config = json5::from_str(&contents).map_err(|e| {
        UStatus::fail_with_code(
            UCode::INTERNAL,
            format!("Unable to parse config file: {e:?}"),
        )
    })?;
    for mqtt_config in &mut config.transports.mqtt {
        mqtt_config.load_mqtt_details().unwrap();
    }

    let subscription_path = config.usubscription_config.file_path;
    let usubscription = Arc::new(USubscriptionStaticFile::new(subscription_path));

    // Start the streamer instance.
    let mut streamer = UStreamer::new(
        "up-streamer",
        config.up_streamer_config.message_queue_size,
        usubscription,
    )
    .expect("Failed to create uStreamer");

    let mut endpoints: HashMap<String, Endpoint> = HashMap::new();

    // build all zenoh transports and endpoints
    for zenoh_transport_config in config.transports.zenoh.clone() {
        let zenoh_config = ZenohConfig::from_file(zenoh_transport_config.config_file).unwrap();
        let uri = UUri::try_from_parts(
            &zenoh_transport_config.authority,
            config.streamer_uuri.ue_id,
            config.streamer_uuri.ue_version_major,
            0,
        )
        .unwrap();
        let zenoh_transport: Arc<dyn UTransport> = Arc::new(
            UPTransportZenoh::new(zenoh_config, uri)
                .await
                .expect("Unable to initialize Zenoh UTransport"),
        );
        let endpoint = Endpoint::new(
            &zenoh_transport_config.endpoint,
            &zenoh_transport_config.authority,
            zenoh_transport.clone(),
        );
        if endpoints
            .insert(zenoh_transport_config.endpoint.clone(), endpoint)
            .is_some()
        {
            return Err(UStatus::fail_with_code(
                UCode::INVALID_ARGUMENT,
                format!(
                    "Duplicate endpoint name found: {}",
                    zenoh_transport_config.endpoint
                ),
            ));
        }
    }

    // build all mqtt transports and endpoints
    for mqtt_transport_config in config.transports.mqtt.clone() {
        let mqtt_options = MqttClientOptions {
            broker_uri: mqtt_transport_config.mqtt_details.clone().unwrap().hostname
                + ":"
                + &mqtt_transport_config.mqtt_details.unwrap().port.to_string(),
            ..Default::default()
        };
        let mqtt5_transport = Mqtt5Transport::new(
            TransportMode::InVehicle,
            mqtt_options,
            mqtt_transport_config.authority.clone(),
        )
        .await?;
        mqtt5_transport.connect().await?;
        let mqtt5_transport: Arc<dyn UTransport> = Arc::new(mqtt5_transport);
        let endpoint = Endpoint::new(
            &mqtt_transport_config.endpoint,
            &mqtt_transport_config.authority,
            mqtt5_transport,
        );
        if endpoints
            .insert(mqtt_transport_config.endpoint.clone(), endpoint)
            .is_some()
        {
            return Err(UStatus::fail_with_code(
                UCode::INVALID_ARGUMENT,
                format!(
                    "Duplicate endpoint name found: {}",
                    mqtt_transport_config.endpoint
                ),
            ));
        }
    }

    // set up the endpoint forwarding for zenoh
    // this has to happen after all endpoints have been set up
    for zenoh in config.transports.zenoh {
        for forwarding in zenoh.forwarding {
            let left_endpoint = endpoints.get(&zenoh.endpoint).unwrap();
            let right_endpoint = endpoints.get(&forwarding).unwrap();
            streamer
                .add_forwarding_rule(left_endpoint.to_owned(), right_endpoint.to_owned())
                .await
                .expect("Could not add forwarding rule from {zenoh.endpoint} to {forwarding}");
        }
    }

    // set up the endpoint forwarding for mqtt
    for mqtt in config.transports.mqtt {
        for forwarding in mqtt.forwarding {
            let left_endpoint = endpoints.get(&mqtt.endpoint).unwrap();
            let right_endpoint = endpoints.get(&forwarding).unwrap();
            streamer
                .add_forwarding_rule(left_endpoint.to_owned(), right_endpoint.to_owned())
                .await
                .expect("Could not add forwarding rule from {mqtt.endpoint} to {forwarding}");
        }
    }

    thread::park();

    Ok(())
}
