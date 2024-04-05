use std::collections::HashSet;

use im::HashMap;
use model::base_types::{MaintenanceCounter, VehicleId};

use crate::tour::Tour;

#[derive(Clone)]
pub struct Transition {
    cycles: Vec<TransitionCycle>,
    total_maintenance_violation: MaintenanceCounter,
}

impl Transition {
    pub fn one_cylce_per_vehicle(tours: &HashMap<VehicleId, Tour>) -> Transition {
        let mut total_maintenance_violation = 0;
        let cycles = tours
            .iter()
            .map(|(&vehicle_id, tour)| {
                let maintenance_violation = tour.maintenance_counter().max(0);
                total_maintenance_violation += maintenance_violation;
                TransitionCycle::new(vec![vehicle_id], maintenance_violation)
            })
            .collect();

        Transition {
            cycles,
            total_maintenance_violation,
        }
    }

    pub fn one_cluster_per_maintenance(tours: &HashMap<VehicleId, Tour>) -> Transition {
        let mut sorted_clusters: Vec<(Vec<VehicleId>, MaintenanceCounter)> = Vec::new(); // TODO Use BTreeMap
        let mut sorted_unassigned_vehicles: Vec<VehicleId> = Vec::new(); // all none maintenance
                                                                         // vehicles sorted by
                                                                         // maintenance counter in descending order

        for (vehicle_id, tour) in tours.iter() {
            if tour.maintenance_counter() < 0 {
                sorted_clusters.push((vec![*vehicle_id], tour.maintenance_counter()));
            } else {
                sorted_unassigned_vehicles.push(*vehicle_id);
            }
        }

        sorted_unassigned_vehicles
            .sort_by_key(|&vehicle_id| -tours.get(&vehicle_id).unwrap().maintenance_counter());
        sorted_clusters.sort_by_key(|&(_, maintenance_counter)| maintenance_counter);

        for vehicle in sorted_unassigned_vehicles {
            let maintenance_counter_of_tour = tours.get(&vehicle).unwrap().maintenance_counter();

            let best_cluster_opt = sorted_clusters.iter_mut().find(|(_, maintenance_counter)| {
                *maintenance_counter + maintenance_counter_of_tour <= 0
            });
            match best_cluster_opt {
                Some((best_cluster, maintenance_counter)) => {
                    best_cluster.push(vehicle);
                    *maintenance_counter += maintenance_counter_of_tour;
                }
                None => {
                    let last_cluster_opt = sorted_clusters.last_mut();
                    match last_cluster_opt {
                        Some((last_cluster, maintenance_counter)) => {
                            last_cluster.push(vehicle);
                            *maintenance_counter += maintenance_counter_of_tour;
                        }
                        None => {
                            sorted_clusters.push((vec![vehicle], maintenance_counter_of_tour));
                        }
                    }
                }
            }
            sorted_clusters.sort_by_key(|&(_, maintenance_counter)| maintenance_counter);
        }

        let mut total_maintenance_violation = 0;
        let cycles = sorted_clusters
            .into_iter()
            .map(|(vehicles, maintenance_counter)| {
                let maintenance_violation = maintenance_counter.max(0);
                total_maintenance_violation += maintenance_violation;
                TransitionCycle::new(vehicles, maintenance_violation)
            })
            .collect();
        Transition {
            cycles,
            total_maintenance_violation,
        }
    }

    pub fn maintenance_violation(&self) -> MaintenanceCounter {
        self.total_maintenance_violation
    }

    pub fn verify_consistency(&self, tours: &HashMap<VehicleId, Tour>) {
        // each vehicle is present in exactly one cycle
        let cycles: Vec<VehicleId> = self
            .cycles
            .iter()
            .flat_map(|transition_cycle| transition_cycle.cycle.iter().cloned())
            .collect();
        assert_eq!(cycles.len(), tours.len());
        let vehicles_from_tours: HashSet<VehicleId> = tours.keys().cloned().collect();
        let vehicles_from_cycles: HashSet<VehicleId> = cycles.iter().cloned().collect();
        assert_eq!(vehicles_from_tours, vehicles_from_cycles);

        // verify maintenance violations
        let mut computed_total_maintenance_violation = 0;
        for transition_cycle in self.cycles.iter() {
            let maintenance_counter: MaintenanceCounter = transition_cycle
                .cycle
                .iter()
                .map(|&vehicle_id| tours.get(&vehicle_id).unwrap().maintenance_counter())
                .sum();
            let computed_maintenance_violation = maintenance_counter.max(0);
            assert_eq!(
                computed_maintenance_violation,
                transition_cycle.maintenance_violation
            );
            computed_total_maintenance_violation += computed_maintenance_violation;
        }
        assert_eq!(
            computed_total_maintenance_violation,
            self.total_maintenance_violation
        );
    }
}

#[derive(Debug, Clone)]
pub struct TransitionCycle {
    cycle: Vec<VehicleId>,
    maintenance_violation: MaintenanceCounter,
}

impl TransitionCycle {
    pub fn new(
        cycle: Vec<VehicleId>,
        maintenance_violation: MaintenanceCounter,
    ) -> TransitionCycle {
        TransitionCycle {
            cycle,
            maintenance_violation,
        }
    }
}