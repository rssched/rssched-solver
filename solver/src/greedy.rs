use crate::Solution;
use crate::Solver;
use objective_framework::Objective;
use model::base_types::VehicleId;
use model::config::Config;
use model::network::Network;
use model::vehicle_types::VehicleTypes;
use solution::path::Path;
use solution::Schedule;
use std::sync::Arc;

pub struct Greedy {
    vehicles: Arc<VehicleTypes>,
    network: Arc<Network>,
    config: Arc<Config>,
    objective: Arc<Objective<Schedule>>,
}

impl Solver for Greedy {
    fn initialize(
        vehicles: Arc<VehicleTypes>,
        network: Arc<Network>,
        config: Arc<Config>,
        objective: Arc<Objective<Schedule>>,
    ) -> Greedy {
        Greedy {
            vehicles,
            network,
            config,
            objective,
        }
    }

    fn solve(&self) -> Solution {
        let mut schedule = Schedule::empty(
            self.vehicles.clone(),
            self.network.clone(),
            self.config.clone(),
        );

        while let Some(service_trip) = self
            .network
            .service_nodes()
            .find(|s| !schedule.is_fully_covered(*s))
        {
            let vehicle_candidates: Vec<VehicleId> = schedule
                .vehicles_iter()
                .filter(|&v| {
                    match schedule.tour_of(v).unwrap().last_non_depot() {
                        Some(last) => self.network.can_reach(last, service_trip),
                        None => false, // there are not vehicles that only goes from depot to depot
                    }
                })
                .collect();

            // pick the vehicle which tour ends the latest
            let final_candidate = vehicle_candidates.iter().max_by_key(|&&v| {
                let last_trip = schedule.tour_of(v).unwrap().last_non_depot().unwrap();
                self.network.node(last_trip).end_time()
            });

            match final_candidate {
                Some(&v) => {
                    schedule = schedule
                        .add_path_to_vehicle_tour(
                            v,
                            Path::new_from_single_node(service_trip, self.network.clone()),
                        )
                        .unwrap();
                }
                None => {
                    schedule = schedule
                        .spawn_vehicle_for_path(
                            self.vehicles.iter().next().unwrap(),
                            vec![service_trip],
                        )
                        .unwrap();
                }
            }
        }

        schedule = schedule.reassign_end_depots_greedily().unwrap();

        self.objective.evaluate(schedule)
    }
}