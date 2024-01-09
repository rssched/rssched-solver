#[cfg(test)]
#[path = "json_serialisation_tests.rs"]
mod json_serialisation_tests;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use time::{DateTime, Duration};

use crate::base_types::{
    DepotId, Distance, LocationId, Meter, NodeId, PassengerCount, StationSide, TrainLength,
    VehicleTypeId,
};
use crate::config::Config;
use crate::locations::{DeadHeadTrip, Locations};
use crate::network::depot::Depot as ModelDepot;
use crate::network::nodes::Node;
use crate::network::nodes::ServiceTrip as ModelServiceTrip;
use crate::network::Network;
use crate::vehicle_types::VehicleType as ModelVehicleType;
use crate::vehicle_types::VehicleTypes;

type Integer = u32;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct JsonInput {
    vehicle_types: Vec<VehicleType>,
    locations: Vec<Location>,
    depots: Vec<Depot>,
    routes: Vec<Route>,
    service_trips: Vec<ServiceTrip>,
    dead_head_trips: DeadHeadTrips,
    parameters: Parameters,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VehicleType {
    id: String,
    name: String,
    seats: Integer,
    capacity: Integer,
    length: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Location {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Depot {
    id: String,
    location: String,
    capacities: Vec<Capacities>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Capacities {
    vehicle_type: String,
    upper_bound: Integer, //TODO: Allow Inf
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Route {
    id: String,
    line: String,
    origin: String,
    destination: String,
    distance: Integer,
    duration: Integer,
    maximal_formation_length: Option<Integer>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ServiceTrip {
    id: String,
    route: String,
    name: String,
    departure: String,
    passengers: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DeadHeadTrips {
    indices: Vec<String>,
    durations: Vec<Vec<Integer>>,
    distances: Vec<Vec<Integer>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Parameters {
    shunting: Shunting,
    defaults: Defaults,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Shunting {
    minimal_duration: Integer,
    dead_head_trip_duration: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Defaults {
    maximal_formation_length: Integer,
}

pub fn load_rolling_stock_problem_instance_from_json(
    input_data: serde_json::Value,
) -> (Arc<VehicleTypes>, Arc<Network>, Arc<Config>) {
    let json_input = serde_json::from_value(input_data).unwrap();
    let locations = Arc::new(create_locations(&json_input));
    let vehicle_types = Arc::new(create_vehicle_types(&json_input));
    let config = Arc::new(create_config(&json_input));
    let network = Arc::new(create_network(
        &json_input,
        locations.clone(),
        config.clone(),
    ));
    (vehicle_types, network, config)
}

fn create_locations(json_input: &JsonInput) -> Locations {
    let mut stations: HashSet<LocationId> = HashSet::new();
    let mut dead_head_trips: HashMap<LocationId, HashMap<LocationId, DeadHeadTrip>> =
        HashMap::new();

    // add stations
    for location in &json_input.locations {
        stations.insert(LocationId::from(&location.id));
    }

    // add dead head trips
    for (i, origin) in json_input.dead_head_trips.indices.iter().enumerate() {
        let origin_station = LocationId::from(origin);
        let mut destination_map: HashMap<LocationId, DeadHeadTrip> = HashMap::new();
        for (j, destination) in json_input.dead_head_trips.indices.iter().enumerate() {
            destination_map.insert(
                LocationId::from(destination),
                DeadHeadTrip::new(
                    Distance::from_meter(json_input.dead_head_trips.distances[i][j] as u64),
                    Duration::from_seconds(json_input.dead_head_trips.durations[i][j]),
                    StationSide::Back,  // TODO: Read this from json
                    StationSide::Front, // TODO: Read this from json
                ),
            );
        }
        dead_head_trips.insert(origin_station, destination_map);
    }

    Locations::new(stations, dead_head_trips)
}

fn create_vehicle_types(json_input: &JsonInput) -> VehicleTypes {
    let vehicle_types: Vec<ModelVehicleType> = json_input
        .vehicle_types
        .iter()
        .map(|unit_type| {
            ModelVehicleType::new(
                VehicleTypeId::from(&unit_type.id),
                unit_type.name.clone(),
                unit_type.seats as PassengerCount,
                unit_type.capacity as PassengerCount,
                unit_type.length as TrainLength,
            )
        })
        .collect();

    VehicleTypes::new(vehicle_types)
}

fn create_config(json_input: &JsonInput) -> Config {
    Config::new(
        Duration::from_seconds(json_input.parameters.shunting.minimal_duration),
        Duration::from_seconds(json_input.parameters.shunting.dead_head_trip_duration),
        Distance::from_meter(json_input.parameters.defaults.maximal_formation_length as u64),
    )
}

fn create_network(
    json_input: &JsonInput,
    locations: Arc<Locations>,
    config: Arc<Config>,
) -> Network {
    let depots = create_depots(json_input, &locations);
    let service_trips = create_service_trip(json_input, &locations);
    let maintenance_slots = vec![]; //TODO: add maintenance nodes
    Network::new(depots, service_trips, maintenance_slots, config, locations)
}

fn create_depots(json_input: &JsonInput, loc: &Locations) -> Vec<ModelDepot> {
    json_input
        .depots
        .iter()
        .map(|depot| {
            let id = DepotId::from(&depot.id);
            let location = loc.get_location(LocationId::from(&depot.location));
            let mut capacities: HashMap<VehicleTypeId, Option<PassengerCount>> = HashMap::new();
            for capacity in &depot.capacities {
                capacities.insert(
                    VehicleTypeId::from(&capacity.vehicle_type),
                    Some(capacity.upper_bound as PassengerCount), // TODO: Accept Inf and map it to
                                                                  // None
                );
            }
            ModelDepot::new(id, location, capacities.clone())
        })
        .collect()
}

fn create_service_trip(json_input: &JsonInput, locations: &Locations) -> Vec<ModelServiceTrip> {
    json_input
        .service_trips
        .iter()
        .map(|service_trip| {
            let route = json_input
                .routes
                .iter()
                .find(|route| route.id == service_trip.route)
                .unwrap();
            let departure = DateTime::new(&service_trip.departure);
            Node::create_service_trip(
                NodeId::from(&service_trip.id),
                locations.get_location(LocationId::from(&route.origin)),
                locations.get_location(LocationId::from(&route.destination)),
                departure,
                departure + Duration::from_seconds(route.duration),
                StationSide::Front, // TODO: Read this from json
                StationSide::Front, // TODO: Read this from json
                Distance::from_meter(route.distance as Meter),
                service_trip.passengers as PassengerCount,
                service_trip.name.clone(),
            )
        })
        .collect()
}
