# Randomness Beacon Consensus Module

This module contains the client code required to bridge to the drand randomness beacon. Specifically, it contains the Gossipsub network implementation that collators use to ingest new pulses from drand.


## Build

`cargo build`

## Test

Unit tests can be run with `cargo test`.

To run integration tests, use the `e2e` feature: `cargo test --features "e2e"`

## Integration 

``` rust
// configure gossipsub for the libp2p network
let state = Arc::new(Mutex::new(GossipsubState { pulses: vec![] }));
let local_identity: sc_network_types::ed25519::Keypair =
    config.network.node_key.clone().into_keypair()?;
let local_identity: libp2p::identity::ed25519::Keypair = local_identity.into();
let local_identity: libp2p::identity::Keypair = local_identity.into();
// dig TXT _dnsaddr.api.drand.sh
let maddr1: libp2p::Multiaddr =
    "/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
        .parse()
        .expect("The string is a well-formatted multiaddress. qed.");
let maddr2: libp2p::Multiaddr =
    "/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y"
        .parse()
        .expect("The string is a well-formatted multiaddress. qed.");

if let Ok(mut gossipsub) = GossipsubNetwork::new(&local_identity, state.clone(), GossipsubConfig::default()) {
    // Spawn the gossipsub network task
    task_manager.spawn_handle().spawn(
        "gossipsub-network",
        None,
        async move {
            if let Err(e) =
                gossipsub.run(DRAND_QUICKNET_PUBSUB_TOPIC, vec![&maddr1, &maddr2], None).await
            {
                log::error!("Failed to run gossipsub network: {:?}", e);
            }
        }
        .boxed(),
    );
}
// END GOSSIPSUB CONFIG

```

then setup the inherent

``` rust
if role.is_authority() {
    let proposer_factory = sc_basic_authorship::ProposerFactory::new(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool.clone(),
        prometheus_registry.as_ref(),
        telemetry.as_ref().map(|x| x.handle()),
    );

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

    let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
        StartAuraParams {
            slot_duration,
            client: client.clone(),
            select_chain,
            block_import,
            proposer_factory,
            create_inherent_data_providers: move |_, ()| {
                
                let shared_state = state.clone();

                async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );

                    let mut data_lock =
                        shared_state.lock().expect("Shared state lock poisoned");

                    let pulses: Vec<OpaquePulse> = data_lock.clone().pulses;
                    let data: Vec<Vec<u8>> =
                        pulses.iter().map(|pulse| pulse.serialize_to_vec()).collect::<Vec<_>>();
                    let beacon =
                        sp_consensus_randomness_beacon::inherents::InherentDataProvider::new(
                            data,
                        );
                    data_lock.pulses.clear();

                    Ok((slot, timestamp, beacon))
                }
            },
            force_authoring,
            backoff_authoring_blocks,
            keystore: keystore_container.keystore(),
            sync_oracle: sync_service.clone(),
            justification_sync_link: sync_service.clone(),
            block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
            max_block_proposal_slot_portion: None,
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            compatibility_mode: Default::default(),
        },
    )?;
}
```

## License

Apache-2.0

