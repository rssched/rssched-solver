pub(crate) mod greedy_1;
pub(crate) mod greedy_2;
pub(crate) mod greedy_3;
pub(crate) mod local_search;

use crate::schedule::Schedule;
use sbb_model::{config::Config, network::Network, vehicle_types::VehicleTypes};
use std::sync::Arc;

pub(crate) trait Solver {
    fn initialize(config: Arc<Config>, vehicle_types: Arc<VehicleTypes>, nw: Arc<Network>) -> Self;

    fn solve(&self) -> Schedule;
}