//RUST_BACKTRACE=1 cargo test scenarios106 -- --nocapture

use super::{mock_protocol_controller::MockProtocolController, tools};
use crate::{start_consensus_controller, timeslots};
use crypto::hash::Hash;
use std::collections::HashSet;
use time::UTime;

#[tokio::test]
async fn test_consensus_sends_block_to_peer_who_asked_for_it() {
    let node_ids = tools::create_node_ids(2);

    let mut cfg = tools::default_consensus_config(&node_ids);
    cfg.t0 = 1000.into();
    cfg.future_block_processing_max_periods = 50;
    cfg.max_future_processing_blocks = 10;

    // mock protocol
    let (mut protocol_controller, protocol_command_sender, protocol_event_receiver) =
        MockProtocolController::new();

    // launch consensus controller
    let (consensus_command_sender, consensus_event_receiver, consensus_manager) =
        start_consensus_controller(
            cfg.clone(),
            protocol_command_sender.clone(),
            protocol_event_receiver,
        )
        .await
        .expect("could not start consensus controller");

    let start_slot = 3;
    let genesis_hashes = consensus_command_sender
        .get_block_graph_status()
        .await
        .expect("could not get block graph status")
        .genesis_blocks;

    //create test blocks
    let (hasht0s1, t0s1, _) = tools::create_block(&cfg, 0, 1 + start_slot, genesis_hashes.clone());
    let header = t0s1.header.clone();

    // Send the actual block.
    protocol_controller.receive_block(t0s1).await;

    //block t0s1 is propagated
    let hash_list = vec![hasht0s1];
    tools::validate_propagate_block_in_list(
        &mut protocol_controller,
        &hash_list,
        3000 + start_slot * 1000,
    )
    .await;

    // Send the hash
    protocol_controller
        .receive_get_active_block(node_ids[1].1.clone(), hasht0s1)
        .await;

    // Consensus should not ask for the block, so the time-out should be hit.
    tools::validate_send_block(&mut protocol_controller, hasht0s1, 10).await;

    // stop controller while ignoring all commands
    let stop_fut = consensus_manager.stop(consensus_event_receiver);
    tokio::pin!(stop_fut);
    protocol_controller
        .ignore_commands_while(stop_fut)
        .await
        .unwrap();
}
