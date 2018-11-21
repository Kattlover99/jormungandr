use std::sync::Arc;
use super::selection::{self, IsLeading, Selection};

use super::super::{
    clock, BlockchainR, TPoolR, utils::task::{TaskMessageBox}, intercom::{BlockMsg}, secure::NodeSecret,
};

use cardano::config::ProtocolMagic;
use cardano::block::{Block, normal, update, HeaderExtraData, ChainDifficulty, EpochSlotId, BlockVersion, SoftwareVersion, BlockHeaderAttributes, HeaderHash};
use cardano::hdwallet;
use cardano::tx::TxAux;
use cardano::hash::Blake2b256;
use cbor_event::Value;

fn make_block(secret: &NodeSecret, my_pub: &hdwallet::XPub, previous_hash: &HeaderHash, slot_id: EpochSlotId, txs: &[TxAux]) -> normal::Block {
    let pm = ProtocolMagic::default();
    let bver = BlockVersion::new(1,0,0);
    let sver = SoftwareVersion::new(env!("CARGO_PKG_NAME"), 1).unwrap();

    let sig = secret.sign_block();

    let body = normal::Body {
        tx: normal::TxPayload::new(txs.to_vec()),
        ssc: normal::SscPayload::fake(),
        delegation: normal::DlgPayload(Value::U64(0)),
        update: update::UpdatePayload {
            proposal: None,
            votes: Vec::new(),
        },
    };
    let body_proof = normal::BodyProof::generate_from_body(&body);

    let hdr = normal::BlockHeader {
        protocol_magic: pm,
        previous_header: previous_hash.clone(),
        body_proof: body_proof,
        consensus: normal::Consensus {
            slot_id: slot_id,
            leader_key: my_pub.clone(),
            chain_difficulty: ChainDifficulty::from(0u64),
            block_signature: sig,
        },
        extra_data: HeaderExtraData {
            block_version: bver,
            software_version: sver,
            attributes: BlockHeaderAttributes(Value::U64(0)),
            extra_data_proof: Blake2b256::from([0u8;Blake2b256::HASH_SIZE]), // hash of the Extra body data
        },
    };
    normal::Block {
        header: hdr,
        body: body,
        extra: Value::U64(0),
    }
}

pub fn leadership_task(secret: NodeSecret, selection: Arc<Selection>, tpool: TPoolR, blockchain: BlockchainR, clock: clock::Clock, block_task: TaskMessageBox<BlockMsg>)
{
    let my_pub = secret.public.block_publickey;
    loop {
        let d = clock.wait_next_slot();
        let (epoch, idx, next_time) = clock.current_slot().unwrap();
        debug!("slept for {:?} epoch {} slot {} next_slot {:?}", d, epoch.0, idx, next_time);
        let len = {
            let t = tpool.read().unwrap();
            (*t).content.len()
        };

        // TODO in the future "current stake" will be one of the parameter
        let leader = selection::test(&selection, idx as u64);

        if leader == IsLeading::Yes {
            // create a new block to broadcast:
            // * get the transactions to put in the transactions
            // * mint the block
            // * sign it
            let (latest_tip, latest_tip_date) = {
                let b = blockchain.read().unwrap();
                b.get_tip()
            };

            info!("leadership create tpool={} transactions tip={} ({})", len, latest_tip, latest_tip_date);

            let epochslot = EpochSlotId { epoch: epoch.0 as u64, slotid: idx as u16 };
            let block = make_block(&secret, &my_pub, &latest_tip, epochslot, &[]);

            block_task.send_to(
                BlockMsg::LeadershipBlock(
                    Block::MainBlock(block)
                )
            );
        }

    }
}
