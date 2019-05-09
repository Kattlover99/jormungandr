mod utxo;

use self::utxo::Utxo;
use actix_web::{App, Json, Responder, State};
use blockchain::BlockchainR;

pub fn create_handler(
    blockchain: BlockchainR,
) -> impl Fn(&str) -> App<BlockchainR> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/utxo", prefix);
        App::with_state(blockchain.clone())
            .prefix(app_prefix)
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(blockchain: State<BlockchainR>) -> impl Responder {
    let blockchain = blockchain.lock_read();
    let utxos = blockchain
        .multiverse
        .get(&blockchain.get_tip())
        .unwrap()
        .utxos();
    let utxos = utxos.map(Utxo::from).collect::<Vec<_>>();
    Json(utxos)
}
