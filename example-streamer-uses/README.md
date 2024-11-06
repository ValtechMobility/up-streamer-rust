# Examples

## Running examples

## Configuring the environment

Set the following variable in your environment by adding it to the projects .cargo/config.toml file:
The default location for vsomeip should be something like "/usr/local/lib".

```toml
LD_LIBRARY_PATH=<path/to/vsomeip/lib>
```

### Running the `up-linux-streamer` as the Streamer instance

To run one of the examples below and see client and service communicate, you'll need to run the `up-linux-streamer` to bridge between the transports in a terminal. See `up-linux-streamer` README for more details.

Run the linux streamer with the default configuration file from the project root directory:

```bash
cargo run -p up-linux-streamer -- --config up-linux-streamer/DEFAULT_CONFIG.json5
```

This starts the streamer which should now be idle and logging its vsomeip version every 10 seconds. As soon as a client tries to connect with the streamer, the connection will be logged.

### Mechatronics client to high compute service

Launch the `uE_service` example in another terminal. This is the compute service which listens for Zenoh messages.

```bash
cargo run -p example-streamer-uses --bin zenoh_service -- --endpoint tcp/0.0.0.0:7445
```

In this example, the "--endpoint" flag will set the endpoint address upon which the zenoh client will listen. This endpoint must match the one given to the Streamer configuration via the DEFAULT_CONFIG.json5 (or specifically the ZENOH_CONFIG.json5 which the default config references).

Launch the `mE_client` example in another terminal. This is the mechatronics component which sends data through SomeIP

```bash
cargo run -p example-streamer-uses --bin me_client
```

The service and client will run forever. Every second a new message is sent from the mE_client via vsomeip. That vsomeip message is caught and routed over Zenoh to the uE_service. The response makes the same journey in reverse.

It's intended that you see the following in the terminal running the `mE_client`:

> Sending Request message:
UMessage { attributes: MessageField(Some(UAttributes { id: MessageField(Some(UUID { msb: 112888656100425728, lsb: 9811577761723054400, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), type_: UMESSAGE_TYPE_REQUEST, source: MessageField(Some(UUri { authority_name: "me_authority", ue_id: 22136, ue_version_major: 1, resource_id: 0, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), sink: MessageField(Some(UUri { authority_name: "linux", ue_id: 4662, ue_version_major: 1, resource_id: 2198, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), priority: UPRIORITY_CS4, ttl: Some(1000), permission_level: None, commstatus: None, reqid: MessageField(None), token: None, traceparent: None, payload_format: UPAYLOAD_FORMAT_PROTOBUF, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), **payload: Some(b"\n\rme_client@i=3")**, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } }

This is the request message we will send _from_ the mE_client over vsomeip.

Note the payload is listed with `@i=3`. This number is incremented on each send so that we can trace the message back and forth over the `up-linux-streamer`.

You should then see something like this in the `uE_service` terminal:

> ServiceRequestResponder: Received a message: UMessage { attributes: MessageField(Some(UAttributes { id: MessageField(Some(UUID { msb: 112888656100622336, lsb: 10998499817480005337, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), type_: UMESSAGE_TYPE_REQUEST, source: MessageField(Some(UUri { authority_name: "me_authority", ue_id: 257, ue_version_major: 1, resource_id: 0, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), sink: MessageField(Some(UUri { authority_name: "linux", ue_id: 4662, ue_version_major: 1, resource_id: 2198, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), priority: UPRIORITY_CS4, ttl: Some(1000), permission_level: None, commstatus: None, reqid: MessageField(None), token: None, traceparent: None, payload_format: UPAYLOAD_FORMAT_UNSPECIFIED, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), **payload: Some(b"\n\rme_client@i=3")**, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } }

Note that the message we received in the uE_service through the streamer still shows the payload with `@i=3`.

If you again reference the terminal where `mE_client` is running you should see the the response message printed:

> ServiceResponseListener: Received a message: UMessage { attributes: MessageField(Some(UAttributes { id: MessageField(Some(UUID { msb: 112888656101015552, lsb: 9811577761723054400, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), type_: UMESSAGE_TYPE_RESPONSE, source: MessageField(Some(UUri { authority_name: "linux", ue_id: 4662, ue_version_major: 1, resource_id: 2198, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), sink: MessageField(Some(UUri { authority_name: "me_authority", ue_id: 257, ue_version_major: 1, resource_id: 0, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), priority: UPRIORITY_CS4, ttl: None, permission_level: None, commstatus: Some(INTERNAL), reqid: MessageField(Some(UUID { msb: 112888656100425728, lsb: 9811577761723054400, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), token: None, traceparent: None, payload_format: UPAYLOAD_FORMAT_UNSPECIFIED, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } })), **payload: Some(b"\nThe response to the request: me_client@i=3")**, special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } }

Note that the response also contains `@i=3` showing that we have received a response for the original request containing that in the payload.

Further, you will see printed the deserialized `HelloResponse` ProtoBuf object:

> Here we received response: HelloResponse { message: "The response to the request: me_client@i=3", special_fields: SpecialFields { unknown_fields: UnknownFields { fields: None }, cached_size: CachedSize { size: 0 } } }

### High compute service to mechatronics client

Launch the `mE_service` example in another terminal:

```bash
LD_LIBRARY_PATH=$LD_LIBRARY_PATH:<path/to/vsomeip/lib> cargo run --example mE_service
```

Launch the `uE_client` example in another terminal:

```bash
LD_LIBRARY_PATH=$LD_LIBRARY_PATH:<path/to/vsomeip/lib> cargo run --example uE_client -- --endpoint tcp/0.0.0.0:7444
```

We omit a detail explanation of the expected terminal output as it's a mirror of the `Mechatronics client to high compute service` heading above.
