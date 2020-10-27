use crate::{
    node::{LeadershipMode, PersistenceMode},
    test::{
        utils::{self, MeasurementReportInterval, SyncWaitParams},
        Result,
    },
    Context, ScenarioResult,
};
use jormungandr_lib::interfaces::Explorer;
use jormungandr_testing_utils::testing::FragmentSenderSetup;
use rand_chacha::ChaChaRng;

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const LEADER_5: &str = "Leader5";

const PASSIVE: &str = "Passive";

pub fn bft_cascade(mut context: Context<ChaChaRng>) -> Result<ScenarioResult> {
    let scenario_settings = prepare_scenario! {
        "Bft nodes",
        &mut context,
        topology [
            LEADER_1,
            LEADER_2 -> LEADER_1,
            LEADER_3 -> LEADER_2 -> LEADER_1,
            LEADER_4 -> LEADER_3 -> LEADER_2,
            LEADER_5 -> LEADER_4 -> LEADER_3,
        ]
        blockchain {
            consensus = Bft,
            number_of_slots_per_epoch = 60,
            slot_duration = 1,
            leaders = [ LEADER_1, LEADER_2, LEADER_3, LEADER_4,LEADER_5 ],
            initials = [
                account "alice" with   500_000_000,
                account "bob" with  500_000_000,
            ],
        }
    };

    let mut controller = scenario_settings.build(context)?;

    controller.monitor_nodes();

    let leader1 =
        controller.spawn_node(LEADER_1, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader1.wait_for_bootstrap()?;

    let leader2 =
        controller.spawn_node(LEADER_2, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader2.wait_for_bootstrap()?;

    let leader3 =
        controller.spawn_node(LEADER_3, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader3.wait_for_bootstrap()?;

    let leader4 =
        controller.spawn_node(LEADER_4, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader4.wait_for_bootstrap()?;

    let leader5 =
        controller.spawn_node(LEADER_5, LeadershipMode::Leader, PersistenceMode::InMemory)?;
    leader5.wait_for_bootstrap()?;

    let leaders = [&leader1, &leader2, &leader3, &leader4, &leader5];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "bft cascade sync",
        MeasurementReportInterval::Standard,
    )?;

    // let leader5 =
    //     controller.spawn_node_custom(controller.new_spawn_params(PASSIVE).passive().explorer(Explorer{enabled: true}))?;
    // passive.wait_for_bootstrap()?;

    let mut alice = controller.wallet("alice")?;
    let mut bob = controller.wallet("bob")?;

    std::thread::sleep(std::time::Duration::from_secs(60));

    controller.fragment_sender().send_transactions_round_trip(
        40,
        &mut alice,
        &mut bob,
        &leader5,
        1_000.into(),
    )?;

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "bft cascade sync",
        MeasurementReportInterval::Standard,
    )?;

    leader5.shutdown()?;
    leader4.shutdown()?;
    leader3.shutdown()?;
    leader2.shutdown()?;
    leader1.shutdown()?;
    Ok(ScenarioResult::passed())
}
