use crate::blockcfg::Header;
use crate::blockchain::chain::{self, BlockHeaderTriage, HandledBlock};
use crate::intercom::BlockMsg;
use crate::rest::v0::node::stats::StatsCounter;
use crate::utils::task::TaskBroadcastBox;
use chain_core::property::Header as _;

pub fn process(
    blockchain: &chain::BlockchainR,
    bquery: BlockMsg,
    network_broadcast: &mut TaskBroadcastBox<Header>,
    stats_counter: &StatsCounter,
) {
    match bquery {
        BlockMsg::LeadershipBlock(block) => {
            let mut blockchain = blockchain.lock_write();
            match chain::handle_block(&mut blockchain, block, true).unwrap() {
                HandledBlock::Rejected { reason } => {
                    warn!("rejecting node's created block: {:?}", reason);
                }
                HandledBlock::MissingBranchToBlock { to } => {
                    // this is an error because we are in a situation
                    // where the leadership has created a block but we
                    // cannot add it in the blockchain because it is not
                    // connected
                    //
                    // We might want to stop the node at this point as this
                    // display corruption of the blockchain's state or of the
                    // storage
                    error!(
                        "the block cannot be added, missing intermediate blocks to {}",
                        to
                    );
                }
                HandledBlock::Acquired { header } => {
                    info!(
                        "block added successfully to Node's blockchain {}",
                        header.id() ;
                        date = header.date()
                    );
                    debug!("Header: {:?}", header);
                    network_broadcast.send_broadcast(header);
                }
            }
        }
        BlockMsg::Subscribe(reply) => {
            let rx = network_broadcast.add_rx();
            reply.send(rx);
        }
        BlockMsg::AnnouncedBlock(header) => {
            let blockchain = blockchain.lock_read();
            match chain::header_triage(&blockchain, header, false).unwrap() {
                BlockHeaderTriage::NotOfInterest { reason } => {
                    info!("rejecting block announcement: {:?}", reason);
                }
                BlockHeaderTriage::MissingParentOrBranch { to } => {
                    // blocks are missing between the received header and the
                    // common ancestor.
                    //
                    // TODO reply to the network to ask for more blocks
                    info!(
                        "received a loose block ({}), missing parent(s) block(s)",
                        to
                    );
                }
                BlockHeaderTriage::ProcessBlockToState => {
                    info!("Block announcement is interesting, fetch block");
                    // TODO: signal back to the network that the block is interesting
                    // (get block/rquiest block)
                    unimplemented!()
                }
            }
        }
    }
}
