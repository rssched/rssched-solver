use im::{HashMap, HashSet};
use sbb_model::base_types::{DepotId, NodeId, VehicleId, VehicleTypeId};

type DepotUsage = HashMap<(DepotId, VehicleTypeId), (HashSet<VehicleId>, HashSet<VehicleId>)>;

use crate::{
    path::Path, segment::Segment, tour::Tour, train_formation::TrainFormation, vehicle::Vehicle,
    Schedule,
};

impl Schedule {
    pub fn spawn_vehicle_to_replace_dummy_tour(
        &self,
        dummy_id: VehicleId,
        vehicle_type_id: VehicleTypeId,
    ) -> Result<Schedule, String> {
        let nodes: Vec<NodeId> = self
            .dummy_tours
            .get(&dummy_id)
            .unwrap()
            .all_nodes_iter()
            .collect();
        let intermediate_schedule = self.delete_dummy(dummy_id)?;
        intermediate_schedule.spawn_vehicle_for_path(vehicle_type_id, nodes)
    }

    /// Spawn new vehicle.
    /// If path does not start with a depot the vehicle is spawned from the nearest availabe depot
    /// (from the start location of the first trip).
    /// Similarly, if path does not end with a depot the vehicle is spawned to the nearest depot
    /// (from the end location of the last trip).
    /// If no depot is available, an error is returned.
    /// If the depot given in the path in not available, an error is returned.
    pub fn spawn_vehicle_for_path(
        &self,
        vehicle_type_id: VehicleTypeId,
        path_as_vec: Vec<NodeId>,
    ) -> Result<Schedule, String> {
        let nodes = self.add_suitable_start_and_end_depot_to_path(vehicle_type_id, path_as_vec)?;

        let mut vehicles = self.vehicles.clone();
        let mut tours = self.tours.clone();
        let mut train_formations = self.train_formations.clone();
        let mut depot_usage = self.depot_usage.clone();
        let mut vehicle_ids_sorted = self.vehicle_ids_sorted.clone();

        let vehicle_id = VehicleId::from(format!("veh{:03}", self.vehicle_counter).as_str());
        let tour = Tour::new(nodes, self.network.clone())?;
        let vehicle = Vehicle::new(vehicle_id, vehicle_type_id, self.vehicle_types.clone());

        vehicles.insert(vehicle_id, vehicle.clone());
        vehicle_ids_sorted.insert(
            vehicle_ids_sorted
                .binary_search(&vehicle_id)
                .unwrap_or_else(|e| e),
            vehicle_id,
        );

        self.update_train_formation(
            &mut train_formations,
            None,
            Some(vehicle.clone()),
            tour.all_nodes_iter(),
        );

        self.update_depot_usage(&mut depot_usage, &vehicles, &tours, vehicle_id);

        tours.insert(vehicle_id, tour);

        Ok(Schedule {
            vehicles,
            tours,
            train_formations,
            depot_usage,
            dummy_tours: self.dummy_tours.clone(),
            vehicle_ids_sorted,
            dummy_ids_sorted: self.dummy_ids_sorted.clone(),
            vehicle_counter: self.vehicle_counter + 1,
            config: self.config.clone(),
            vehicle_types: self.vehicle_types.clone(),
            network: self.network.clone(),
        })
    }

    /// Delete vehicle (and its tour) from schedule.
    pub fn delete_vehicle(&self, vehicle_id: VehicleId) -> Result<Schedule, String> {
        if self.is_dummy(vehicle_id) {
            return Err(format!(
                "Cannot delete dummy vehicle {} from schedule.",
                vehicle_id
            ));
        }
        let mut vehicles = self.vehicles.clone();
        let mut tours = self.tours.clone();
        let mut train_formations = self.train_formations.clone();
        let mut depot_usage = self.depot_usage.clone();
        let mut vehicle_ids_sorted = self.vehicle_ids_sorted.clone();

        vehicles.remove(&vehicle_id);
        vehicle_ids_sorted.remove(vehicle_ids_sorted.binary_search(&vehicle_id).unwrap());

        tours.remove(&vehicle_id);

        self.update_train_formation(
            &mut train_formations,
            Some(vehicle_id),
            None,
            tours.get(&vehicle_id).unwrap().all_nodes_iter(),
        );

        self.update_depot_usage(&mut depot_usage, &vehicles, &tours, vehicle_id);

        Ok(Schedule {
            vehicles,
            tours,
            train_formations,
            depot_usage,
            dummy_tours: self.dummy_tours.clone(),
            vehicle_ids_sorted,
            dummy_ids_sorted: self.dummy_ids_sorted.clone(),
            vehicle_counter: self.vehicle_counter,
            config: self.config.clone(),
            vehicle_types: self.vehicle_types.clone(),
            network: self.network.clone(),
        })
    }

    /// Delete dummy vehicle (and its tour) from schedule.
    pub fn delete_dummy(&self, dummy: VehicleId) -> Result<Schedule, String> {
        if !self.is_dummy(dummy) {
            return Err(format!(
                "Cannot delete vehicle {} from schedule. It is not a dummy vehicle.",
                dummy
            ));
        }
        let mut dummy_tours = self.dummy_tours.clone();
        let mut dummy_ids_sorted = self.dummy_ids_sorted.clone();

        dummy_tours.remove(&dummy);
        dummy_ids_sorted.remove(dummy_ids_sorted.binary_search(&dummy).unwrap());

        Ok(Schedule {
            vehicles: self.vehicles.clone(),
            tours: self.tours.clone(),
            train_formations: self.train_formations.clone(),
            depot_usage: self.depot_usage.clone(),
            dummy_tours,
            vehicle_ids_sorted: self.vehicle_ids_sorted.clone(),
            dummy_ids_sorted,
            vehicle_counter: self.vehicle_counter,
            config: self.config.clone(),
            vehicle_types: self.vehicle_types.clone(),
            network: self.network.clone(),
        })
    }

    /// Add a path to the tour of a vehicle. If the path causes conflicts, the conflicting nodes of
    /// the old tour are removed.
    pub fn add_path_to_vehicle_tour(
        &self,
        vehicle_id: VehicleId,
        path: Path,
    ) -> Result<Schedule, String> {
        let mut tours = self.tours.clone();
        let mut train_formations = self.train_formations.clone();
        let mut depot_usage = self.depot_usage.clone();

        // add vehicle to train_formations for nodes of new path
        self.update_train_formation(
            &mut train_formations,
            None,
            Some(self.vehicles.get(&vehicle_id).cloned().unwrap()),
            path.iter(),
        );

        let (new_tour, removed_path_opt) = tours.get(&vehicle_id).unwrap().insert_path(path);

        // remove vehicle from train formations for nodes of removed path
        if let Some(removed_path) = removed_path_opt {
            self.update_train_formation(
                &mut train_formations,
                Some(vehicle_id),
                None,
                removed_path.iter(),
            );
        }

        self.update_depot_usage(&mut depot_usage, &self.vehicles, &tours, vehicle_id);

        tours.insert(vehicle_id, new_tour);

        Ok(Schedule {
            vehicles: self.vehicles.clone(),
            tours,
            train_formations,
            depot_usage,
            dummy_tours: self.dummy_tours.clone(),
            vehicle_ids_sorted: self.vehicle_ids_sorted.clone(),
            dummy_ids_sorted: self.dummy_ids_sorted.clone(),
            vehicle_counter: self.vehicle_counter,
            config: self.config.clone(),
            vehicle_types: self.vehicle_types.clone(),
            network: self.network.clone(),
        })
    }

    /// Reassigns a path (given by a segment and a provider) to the tour of receiver.
    /// Aborts if there are any conflicts.
    pub fn cautious_reassign(
        &self,
        segment: Segment,
        provider: VehicleId,
        receiver: VehicleId,
    ) -> Result<Schedule, String> {
        if self.conflict(segment, receiver).is_some() {
            return Err(format!(
                "There are conflcits. Abort cautious_reassign()! Segment: {} Provider: {} Receiver: {}", segment, provider, receiver
            ));
        }
        self.override_reassign(segment, provider, receiver)
            .map(|tuple| tuple.0)
    }

    /// Tries to insert all nodes of provider's segment into receiver's tour.
    /// Nodes that causes conflcits are rejected and stay in provider's tour.
    /// Nodes that do not cause a conflict are reassigned to the receiver.
    /// Afterwards for each of the new tours the depots are changed to the nearest ones.
    pub fn fit_reassign(
        &self,
        segment: Segment,
        provider: VehicleId,
        receiver: VehicleId,
    ) -> Result<Schedule, String> {
        let mut vehicles = self.vehicles.clone();
        let mut tours = self.tours.clone();
        let mut train_formations = self.train_formations.clone();
        let mut depot_usage = self.depot_usage.clone();
        let mut dummy_tours = self.dummy_tours.clone();
        let mut vehicle_ids_sorted = self.vehicle_ids_sorted.clone();
        let mut dummy_ids_sorted = self.dummy_ids_sorted.clone();

        let (new_tour_provider, new_tour_receiver, moved_nodes) = self.fit_path_into_tour(
            self.tour_of(provider).unwrap().sub_path(segment)?,
            provider,
            receiver,
        );

        self.update_tours(
            &mut vehicles,
            &mut tours,
            &mut train_formations,
            &mut depot_usage,
            &mut dummy_tours,
            &mut vehicle_ids_sorted,
            &mut dummy_ids_sorted,
            Some(provider),
            new_tour_provider,
            receiver,
            new_tour_receiver,
            moved_nodes.iter().copied(),
        );

        Ok(Schedule {
            vehicles,
            tours,
            train_formations,
            depot_usage,
            dummy_tours,
            vehicle_ids_sorted,
            dummy_ids_sorted,
            vehicle_counter: self.vehicle_counter,
            config: self.config.clone(),
            vehicle_types: self.vehicle_types.clone(),
            network: self.network.clone(),
        })
    }

    /// Remove segment from provider's tour and inserts the nodes into the tour of receiver vehicle.
    /// All conflicting nodes are removed from the tour and in the case that there are conflicts
    /// a new dummy tour is created.
    /// Afterwards for each of the new tours the depots are changed to the nearest ones.
    pub fn override_reassign(
        &self,
        segment: Segment,
        provider: VehicleId,
        receiver: VehicleId,
    ) -> Result<(Schedule, Option<VehicleId>), String> {
        let mut vehicles = self.vehicles.clone();
        let mut tours = self.tours.clone();
        let mut dummy_tours = self.dummy_tours.clone();
        let mut train_formations = self.train_formations.clone();
        let mut depot_usage = self.depot_usage.clone();
        let mut vehicle_ids_sorted = self.vehicle_ids_sorted.clone();
        let mut dummy_ids_sorted = self.dummy_ids_sorted.clone();
        let mut dummy_counter = self.vehicle_counter;

        let tour_provider = self.tour_of(provider).unwrap();
        let tour_receiver = self.tour_of(receiver).unwrap();

        // remove segment for provider
        let (shrinked_tour_provider, path) = tour_provider.remove(segment)?;

        let moved_nodes: Vec<NodeId> = path.iter().collect();

        // insert path into tour
        let (new_tour_receiver, replaced_path) = tour_receiver.insert_path(path);

        self.update_tours(
            &mut vehicles,
            &mut tours,
            &mut train_formations,
            &mut depot_usage,
            &mut dummy_tours,
            &mut vehicle_ids_sorted,
            &mut dummy_ids_sorted,
            Some(provider),
            shrinked_tour_provider,
            receiver,
            new_tour_receiver,
            moved_nodes.iter().cloned(),
        );

        let mut new_dummy_opt = None; // for return value

        // insert new dummy tour consisting of conflicting nodes removed from receiver's tour
        if let Some(new_path) = replaced_path {
            let new_dummy = VehicleId::from(format!("dummy{:05}", dummy_counter).as_str());
            new_dummy_opt = Some(new_dummy);

            if self.is_vehicle(receiver) {
                // in this case receiver needs to be removed from the train formations of the
                // removed nodes
                self.update_train_formation(
                    &mut train_formations,
                    Some(receiver),
                    None,
                    new_path.iter(),
                );
            }

            self.add_dummy_tour(&mut dummy_tours, &mut dummy_ids_sorted, new_dummy, new_path);
            dummy_counter += 1;
        }

        Ok((
            Schedule {
                vehicles,
                tours,
                train_formations,
                depot_usage,
                dummy_tours,
                vehicle_ids_sorted,
                dummy_ids_sorted,
                vehicle_counter: dummy_counter,
                config: self.config.clone(),
                vehicle_types: self.vehicle_types.clone(),
                network: self.network.clone(),
            },
            new_dummy_opt,
        ))
    }

    pub fn reassign_end_depots_greedily(&self) -> Result<Schedule, String> {
        let mut tours = self.tours.clone();
        let mut depot_usage = self.depot_usage.clone();

        for vehicle_id in self.vehicle_ids_sorted.iter() {
            let tour = self.tour_of(*vehicle_id).unwrap();
            let last_node_location = self
                .network
                .node(tour.last_non_depot().unwrap())
                .end_location();
            let new_end_depot_node = self
                .network
                .end_depots_sorted_by_distance_from(last_node_location)
                .first()
                .copied()
                .ok_or(format!("Cannot find end depot for vehicle {}.", vehicle_id))?;

            let new_tour = tour.replace_end_depot(new_end_depot_node).unwrap();
            tours.insert(*vehicle_id, new_tour);

            self.update_depot_usage(&mut depot_usage, &self.vehicles, &tours, *vehicle_id);
        }

        Ok(Schedule {
            vehicles: self.vehicles.clone(),
            tours,
            train_formations: self.train_formations.clone(),
            depot_usage,
            dummy_tours: self.dummy_tours.clone(),
            vehicle_ids_sorted: self.vehicle_ids_sorted.clone(),
            dummy_ids_sorted: self.dummy_ids_sorted.clone(),
            vehicle_counter: self.vehicle_counter + 1,
            config: self.config.clone(),
            vehicle_types: self.vehicle_types.clone(),
            network: self.network.clone(),
        })
    }
}

// private methods
impl Schedule {
    /// Reassign vehicles to the new tours.
    /// The depots of the tours are improved.
    /// Updates all relevant data structures.
    /// It assumed that provider (if some) and receiver are part of self.vehicles.
    #[allow(clippy::too_many_arguments)]
    fn update_tours(
        &self,
        vehicles: &mut HashMap<VehicleId, Vehicle>,
        tours: &mut HashMap<VehicleId, Tour>,
        train_formations: &mut HashMap<NodeId, TrainFormation>,
        depot_usage: &mut DepotUsage,
        dummy_tours: &mut HashMap<VehicleId, Tour>,
        vehicle_ids_sorted: &mut Vec<VehicleId>,
        dummy_ids_sorted: &mut Vec<VehicleId>,
        provider: Option<VehicleId>,     // None: there is no provider
        new_tour_provider: Option<Tour>, // None: provider is deleted
        receiver: VehicleId,
        new_tour_receiver: Tour,
        moved_nodes: impl Iterator<Item = NodeId>,
    ) {
        if let Some(provider_id) = provider {
            // update tour of the provider
            match new_tour_provider {
                Some(new_tour) => {
                    self.update_tour(tours, dummy_tours, provider_id, new_tour);
                }
                None => {
                    // there is a provider but no tour -> delete provider
                    if self.is_dummy(provider_id) {
                        dummy_tours.remove(&provider_id); // old_dummy_tour is completely removed
                        dummy_ids_sorted
                            .remove(dummy_ids_sorted.binary_search(&provider_id).unwrap());
                    } else if self.is_vehicle(provider_id) {
                        vehicles.remove(&provider_id);
                        tours.remove(&provider_id); // old_tour is completely removed
                        vehicle_ids_sorted
                            .remove(vehicle_ids_sorted.binary_search(&provider_id).unwrap());
                    }
                }
            }
            self.update_depot_usage(depot_usage, vehicles, tours, provider_id);
        }

        // update extended tour of the receiver
        self.update_tour(tours, dummy_tours, receiver, new_tour_receiver);
        self.update_depot_usage(depot_usage, vehicles, tours, receiver);

        // update train_formations
        let receiver_vehicle = self.vehicles.get(&receiver).cloned();
        self.update_train_formation(train_formations, provider, receiver_vehicle, moved_nodes);
    }

    fn update_tour(
        &self,
        tours: &mut HashMap<VehicleId, Tour>,
        dummy_tours: &mut HashMap<VehicleId, Tour>,
        vehicle: VehicleId,
        new_tour: Tour,
    ) {
        if self.is_dummy(vehicle) {
            dummy_tours.insert(vehicle, new_tour);
        } else {
            tours.insert(
                vehicle,
                self.improve_depots_of_tour(new_tour, self.vehicle_type_of(vehicle)),
            );
        }
    }

    fn update_train_formation(
        &self,
        train_formations: &mut HashMap<NodeId, TrainFormation>,
        provider: Option<VehicleId>,       // None: only add receiver
        receiver_vehicle: Option<Vehicle>, // None: only delete provider
        moved_nodes: impl Iterator<Item = NodeId>,
    ) {
        for node in moved_nodes {
            if self.network.node(node).is_depot() {
                continue;
            }
            train_formations.insert(
                node,
                self.vehicle_replacement_in_train_formation(
                    provider,
                    receiver_vehicle.clone(),
                    node,
                )
                .unwrap(),
            );
        }
    }

    /// Replace a vehicle in the train formation of a node.
    /// Provider or receiver can be a dummy vehicle.
    fn vehicle_replacement_in_train_formation(
        &self,
        provider: Option<VehicleId>,
        receiver_vehicle: Option<Vehicle>,
        node: NodeId,
    ) -> Result<TrainFormation, String> {
        let old_formation = self
            .train_formations
            .get(&node)
            .unwrap_or_else(|| panic!("Node {} has no train formations.", node));

        match receiver_vehicle {
            Some(receiver_vh) if !self.is_dummy(receiver_vh.id()) => {
                match provider {
                    Some(prov) if !self.is_dummy(prov) => {
                        // both are real vehicles
                        old_formation.replace(prov, receiver_vh)
                    }
                    _ => {
                        // provider is None or dummy
                        Ok(old_formation.add_at_tail(receiver_vh))
                    }
                }
            }
            _ => {
                // receiver_vehicle is None or dummy
                match provider {
                    Some(prov) if !self.is_dummy(prov) => {
                        // provider is real vehicle
                        old_formation.remove(prov)
                    }
                    _ => {
                        // provider is None or dummy
                        Ok(old_formation.clone())
                    }
                }
            }
        }

        // if receiver_vehicle.is_none() || self.is_dummy(receiver_vehicle.as_ref().unwrap().id()) {
        // if provider.is_none() || self.is_dummy(provider.unwrap()) {
        // Ok(old_formation.clone())
        // } else {
        // old_formation.remove(provider.unwrap())
        // }
        // } else

        // if provider.is_none() || self.is_dummy(provider.unwrap()) {
        // Ok(old_formation.add_at_tail(receiver_vehicle.unwrap()))
        // } else {
        // old_formation.replace(provider.unwrap(), receiver_vehicle.unwrap())
        // }
    }

    /// Updates the provided depot_usage data structure.
    /// The vehicle is removed from the old depots (if it was a real vehicle in self).
    /// The vehicle is added to the new depots if vehicle is real new schedule (given by vehicles) and new_tour is Some.
    fn update_depot_usage(
        &self,
        depot_usage: &mut DepotUsage,
        vehicles: &HashMap<VehicleId, Vehicle>,
        tours: &HashMap<VehicleId, Tour>,
        vehicle_id: VehicleId,
    ) {
        match vehicles.get(&vehicle_id) {
            Some(vehicle) => self.update_depot_usage_assuming_no_dummies(
                depot_usage,
                vehicle.clone(),
                tours.get(&vehicle_id),
            ),
            None => {
                // vehicle is dummy in new schedule
                if let Some(vehicle) = self.vehicles.get(&vehicle_id) {
                    self.update_depot_usage_assuming_no_dummies(depot_usage, vehicle.clone(), None)
                }
            }
        }
    }

    /// Updates the provided depot_usage data structure.
    /// The vehicle is removed from the old depots (if it was a real vehicle in self).
    /// The vehicle is added to the new depots if new_tour is Some.
    /// It is assumed that the vehicle is no dummy for the new schedule.
    fn update_depot_usage_assuming_no_dummies(
        &self,
        depot_usage: &mut DepotUsage,
        vehicle: Vehicle,
        new_tour: Option<&Tour>,
    ) {
        let new_start_depot_node = new_tour.map(|t| t.start_depot().unwrap());
        let new_end_depot_node = new_tour.map(|t| t.end_depot().unwrap());
        self.update_depot_usage_for_new_start_depot(
            depot_usage,
            vehicle.clone(),
            new_start_depot_node,
        );
        self.update_depot_usage_for_new_end_depot(depot_usage, vehicle, new_end_depot_node);
    }

    fn update_depot_usage_for_new_end_depot(
        &self,
        depot_usage: &mut DepotUsage,
        vehicle: Vehicle,
        new_end_depot_node: Option<NodeId>, // if None, the vehicle is deleted
    ) {
        let vehicle_type = vehicle.type_id();
        let vehicle_id = vehicle.id();

        if self.is_vehicle(vehicle_id) {
            let old_depot = self
                .network
                .get_depot_id(self.tour_of(vehicle_id).unwrap().end_depot().unwrap());
            depot_usage
                .entry((old_depot, vehicle_type))
                .and_modify(|e| {
                    e.1.remove(&vehicle_id).unwrap();
                });
        }

        if let Some(end_depot_node) = new_end_depot_node {
            let new_depot = self.network.get_depot_id(end_depot_node);
            depot_usage
                .entry((new_depot, vehicle_type))
                .and_modify(|e| {
                    e.1.insert(vehicle_id);
                });
        }
    }

    fn update_depot_usage_for_new_start_depot(
        &self,
        depot_usage: &mut DepotUsage,
        vehicle: Vehicle,
        new_start_depot_node: Option<NodeId>, // if None, the vehicle is deleted
    ) {
        let vehicle_type = vehicle.type_id();
        let vehicle_id = vehicle.id();

        if self.is_vehicle(vehicle_id) {
            let old_depot = self
                .network
                .get_depot_id(self.tour_of(vehicle_id).unwrap().start_depot().unwrap());
            depot_usage
                .entry((old_depot, vehicle_type))
                .and_modify(|e| {
                    e.0.remove(&vehicle_id).unwrap();
                });
        }

        if let Some(start_depot_node) = new_start_depot_node {
            let new_depot = self.network.get_depot_id(start_depot_node);
            depot_usage
                .entry((new_depot, vehicle_type))
                .and_modify(|e| {
                    e.0.insert(vehicle_id);
                });
        }
    }

    fn add_dummy_tour(
        &self,
        dummy_tours: &mut HashMap<VehicleId, Tour>,
        dummy_ids_sorted: &mut Vec<VehicleId>,
        dummy_id: VehicleId,
        path: Path,
    ) {
        let dummy_tour = Tour::new_dummy_by_path(path, self.network.clone());
        dummy_tours.insert(dummy_id, dummy_tour);
        dummy_ids_sorted.insert(
            dummy_ids_sorted
                .binary_search(&dummy_id)
                .unwrap_or_else(|e| e),
            dummy_id,
        );
    }

    /// go through the path that should be inserted without causing conflcits.
    /// As dead_head_trips might be longer than service trips we do not iterate over all nodes
    /// individually but instead cut the path into maximal segments that could be reassigned.
    ///
    /// Hence, we iteratively consider the first node of the remaining_path as the start of a
    /// segment and take the biggest segment that can be reassigned.
    /// Afterwards this segment is removed.
    ///
    /// Assumes that path is a sub path of the tour of provider.
    ///
    /// Returns: (new_tour_provider, new_tour_receiver, moved_nodes)
    /// None for new_tour_provider means there is no tour left.
    fn fit_path_into_tour(
        &self,
        path: Path,
        provider: VehicleId,
        receiver: VehicleId,
    ) -> (Option<Tour>, Tour, Vec<NodeId>) {
        let mut new_tour_provider = Some(self.tour_of(provider).unwrap().clone());
        let mut new_tour_receiver = self.tour_of(receiver).unwrap().clone();
        let mut remaining_path = Some(path);
        let mut moved_nodes = Vec::new();

        while let Some(path) = remaining_path {
            let sub_segment_start = path.first();
            let (end_pos, sub_segment_end) =
                match new_tour_receiver.latest_not_reaching_node(sub_segment_start) {
                    None => (path.len() - 1, path.last()),
                    Some(pos) => {
                        // the segment can only be inserted before the blocker
                        let blocker = new_tour_receiver.nth_node(pos).unwrap();
                        // consider all nodes that arrive before the departure of the blocker
                        // test all of them if they can reach the blocker.
                        // test all of them if this segment could be removed.
                        // take the latest node of those.
                        // If empty this segment will fail, so return path.first()
                        path.iter()
                            .enumerate()
                            .map_while(|(i, n)| {
                                if self.network.node(n).end_time()
                                    > self.network.node(blocker).start_time()
                                {
                                    None
                                } else {
                                    Some((i, n))
                                }
                            })
                            .filter(|(_, n)| self.network.can_reach(*n, blocker))
                            .filter(|(_, n)| {
                                new_tour_provider
                                    .as_ref()
                                    .unwrap()
                                    .check_removable(Segment::new(sub_segment_start, *n))
                                    .is_ok()
                            })
                            .last()
                            .unwrap_or((0, path.first()))
                    }
                };

            let mut node_sequence = path.consume();
            remaining_path =
                Path::new_trusted(node_sequence.split_off(end_pos + 1), self.network.clone());
            let sub_segment = Segment::new(sub_segment_start, sub_segment_end);
            let remove_result = new_tour_provider.as_ref().unwrap().remove(sub_segment);

            if remove_result.is_err() {
                continue;
            }

            let (new_tour_provider_candidate, path_for_insertion) = remove_result.unwrap();

            // test if inserting sub_segment would cause any conflicts (or fail for other reasons
            if new_tour_receiver.conflict(sub_segment).is_some() {
                continue;
            }
            let (receiver, _) = new_tour_receiver.insert_path(path_for_insertion);

            new_tour_provider = new_tour_provider_candidate;
            new_tour_receiver = receiver;
            moved_nodes.extend(node_sequence);
        }
        (new_tour_provider, new_tour_receiver, moved_nodes)
    }
    fn improve_depots_of_tour(&self, tour: Tour, vehicle_type_id: VehicleTypeId) -> Tour {
        let first_non_depot = tour.first_non_depot().unwrap();
        let new_start_depot = self
            .find_best_start_depot_for_spawning(vehicle_type_id, first_non_depot)
            .unwrap();
        let intermediate_tour = if new_start_depot != tour.start_depot().unwrap() {
            tour.replace_start_depot(new_start_depot).unwrap()
        } else {
            tour
        };

        let last_non_depot = intermediate_tour.last_non_depot().unwrap();
        let new_end_depot = self
            .find_best_end_depot_for_despawning(vehicle_type_id, last_non_depot)
            .unwrap();
        if new_end_depot != intermediate_tour.end_depot().unwrap() {
            intermediate_tour.replace_end_depot(new_end_depot).unwrap()
        } else {
            intermediate_tour
        }
    }

    fn add_suitable_start_and_end_depot_to_path(
        &self,
        vehicle_type_id: VehicleTypeId,
        mut nodes: Vec<NodeId>,
    ) -> Result<Vec<NodeId>, String> {
        let first_node = *nodes.first().unwrap();
        let last_node = *nodes.last().unwrap();

        // check if depot is available
        if self.network.node(first_node).is_depot()
            && !self.can_depot_spawn_vehicle(first_node, vehicle_type_id)
        {
            return Err(format!(
                "Cannot spawn vehicle of type {} for tour {:?} at start_depot {}. No capacities available.",
                vehicle_type_id,
                nodes, first_node,
            ));
        }

        // TODO check if vehicle can be despawned at given end_depot

        // if path does not start with a depot, insert the nearest available start_depot
        if !self.network.node(first_node).is_depot() {
            match self.find_best_start_depot_for_spawning(vehicle_type_id, first_node) {
                Ok(depot) => nodes.insert(0, depot),
                Err(e) => return Err(e),
            };
        }

        // if path does not end with a depot, insert the nearest available end_depot
        if !self.network.node(last_node).is_depot() {
            match self.find_best_end_depot_for_despawning(vehicle_type_id, last_node) {
                Ok(depot) => nodes.push(depot),
                Err(e) => return Err(e),
            };
        }

        Ok(nodes)
    }

    fn find_best_start_depot_for_spawning(
        &self,
        vehicle_type_id: VehicleTypeId,
        first_node: NodeId,
    ) -> Result<NodeId, String> {
        let start_location = self.network.node(first_node).start_location();
        let start_depot = self
            .network
            .start_depots_sorted_by_distance_to(start_location)
            .iter()
            .copied()
            .find(|depot| self.can_depot_spawn_vehicle(*depot, vehicle_type_id));
        match start_depot {
            Some(depot) => Ok(depot),
            None => Err(format!(
                "Cannot spawn vehicle of type {} for start_node {}. No start_depot available.",
                vehicle_type_id, first_node,
            )),
        }
    }

    fn find_best_end_depot_for_despawning(
        &self,
        vehicle_type_id: VehicleTypeId,
        last_node: NodeId,
    ) -> Result<NodeId, String> {
        let end_location = self.network.node(last_node).end_location();
        let end_depot = self
            .network
            .end_depots_sorted_by_distance_from(end_location)
            .first()
            .copied();
        // .find(|depot| self.can_depot_despawn_vehicle(*depot, vehicle_type_id)); // TODO check if depot can de-spawn vehicle
        match end_depot {
            Some(depot) => Ok(depot),
            None => Err(format!(
                "Cannot de-spawn vehicle of type {} for end_node {}. No end_depot available.",
                vehicle_type_id, last_node,
            )),
        }
    }
}
