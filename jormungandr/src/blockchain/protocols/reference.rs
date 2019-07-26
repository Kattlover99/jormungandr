use crate::blockcfg::{BlockDate, ChainLength, Ledger, Header, Leadership, HeaderHash, LedgerParameters};
use chain_impl_mockchain::multiverse::GCRoot;
use std::sync::Arc;

/// a reference to a block in the blockchain
#[derive(Clone)]
pub struct Ref {
    /// GCRoot holder for the object in the `Multiverse<Ledger>`.
    ledger_gc: Arc<GCRoot>,

    ledger: Arc<Ledger>,

    /// the leadership used to validate the current header's leader
    ///
    /// this object will be shared between different Ref of the same epoch
    epoch_leadership_schedule: Arc<Leadership>,

    /// pointer to the current ledger parameters
    ///
    /// The object will be shared between different Ref of the same epoch
    epoch_ledger_parameters: Arc<LedgerParameters>,

    /// keep the Block header in memory, this will avoid retrieving
    /// the data from the storage if needs be
    header: Header,
}

impl Ref {
    /// create a new `Ref`
    pub fn new(ledger_pointer: GCRoot, ledger: Arc<Ledger>, epoch_leadership_schedule: Arc<Leadership>, epoch_ledger_parameters: Arc<LedgerParameters> , header: Header) -> Self {
        #[cfg(debug_assertions)]
        use std::ops::Deref as _;

        debug_assert!(
            ledger_pointer.deref() == &header.hash(),
            "expect the GCRoot to be for the same `Header`"
        );

        Ref {
            ledger_gc: Arc::new(ledger_pointer),
            ledger,
            epoch_leadership_schedule,
            epoch_ledger_parameters,
            header,
        }
    }

    /// retrieve the header hash of the `Ref`
    pub fn hash(&self) -> &HeaderHash {
        use std::ops::Deref as _;

        self.ledger_gc.deref()
    }

    /// access the reference's parent hash
    pub fn block_parent_hash(&self) -> &HeaderHash {
        self.header().block_parent_hash()
    }

    /// retrieve the block date of the `Ref`
    pub fn block_date(&self) -> &BlockDate {
        self.header().block_date()
    }

    /// retrieve the chain length, the number of blocks created
    /// between the block0 and this block. This is useful to compare
    /// the density of 2 branches.
    pub fn chain_length(&self) -> ChainLength {
        self.header().chain_length()
    }

    /// access the `Header` of the block pointed by this `Ref`
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn ledger(&self) -> &Arc<Ledger> {
        &self.ledger
    }

    pub fn epoch_leadership_schedule(&self) -> &Arc<Leadership> {
        &self.epoch_leadership_schedule
    }

    pub fn epoch_ledger_parameters(&self) -> &Arc<LedgerParameters> {
        &self.epoch_ledger_parameters
    }
}
