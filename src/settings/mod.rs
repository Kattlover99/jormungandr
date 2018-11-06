mod command_arguments;
mod config;
pub mod network;

use std::path::PathBuf;
use cardano::config::GenesisData;
use std::fs::File;
use std::io::Read;
use std;

use exe_common::parse_genesis_data::parse_genesis_data;

pub use self::command_arguments::CommandArguments;
pub use self::config::{Bft, BftConstants, Genesis, GenesisConstants, BftLeader};

pub struct Settings {
    pub cmd_args: CommandArguments,

    pub network: network::Configuration,

    pub genesis_data_config: PathBuf,

    pub consensus: Consensus,
}

#[derive(Debug)]
pub enum Consensus {
    /// BFT consensus
    Bft(config::Bft),
    /// Genesis consensus
    Genesis,
}


impl Settings {
    pub fn load() -> Self {
        let command_arguments = CommandArguments::load();

        let network = network::Configuration {
            peer_nodes: command_arguments.connect_to.clone(),
            listen_to:  command_arguments.listen_addr.clone(),
        };

        let config : config::Config = {
            let mut file = File::open(command_arguments.node_config.clone()).unwrap();
            match serde_yaml::from_reader(&mut file) {
                Err(e) => {
                    println!("config error: {}", e);
                    std::process::exit(1);
                },
                Ok(c) => c,
            }
        };

        let consensus = {
            if let Some(bft) = config.bft {
                Consensus::Bft(bft)
            } else if let Some(genesis) = config.genesis {
                Consensus::Genesis
            } else {
                println!("no consensus algorithm defined");
                std::process::exit(1);
            }
        };

        Settings {
            genesis_data_config: command_arguments.genesis_data_config.clone(),
            network: network,
            cmd_args: command_arguments,
            consensus: consensus,
        }
    }

    pub fn get_log_level(&self) -> log::LevelFilter {
        let log_level = match self.cmd_args.verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        log_level
    }

    pub fn read_genesis_data(&self) -> GenesisData {
        let filepath = &self.cmd_args.genesis_data_config;
        let mut f = File::open(filepath).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap();

        parse_genesis_data(&buffer[..])
    }
}
