use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;

use crate::base_types::StationSide;
use crate::distance::Distance;
use crate::locations::{DeadHeadTrip, Locations, Station};
use crate::time::Duration;

type Integer = u32;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UnitType {
    id: String,
    name: String,
    number_of_seats: Integer,
    capacity_of_passengers: Integer,
    unit_length_in_meter: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Location {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UpperBoundForUnitTypes {
    unit_type: String,
    upper_bound: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Depot {
    id: String,
    location: String,
    upper_bound_for_unit_types: Vec<UpperBoundForUnitTypes>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Route {
    id: String,
    line: String,
    origin: String,
    destination: String,
    travel_distance_in_meter: Integer,
    travel_duration_in_seconds: Integer,
    maximal_formation_length_in_meter: Option<Integer>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ServiceTrip {
    id: String,
    route: String,
    name: String,
    departure_time: String,
    passenger_demand: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DeadHeadTrips {
    indices: Vec<String>,
    travel_time_durations_in_seconds: Vec<Vec<Integer>>,
    distances_in_meter: Vec<Vec<Integer>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ShuntingDurationsInSeconds {
    minimal_duration: Integer,
    dead_head_trip_duration: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Parameters {
    shunting_durations_in_seconds: ShuntingDurationsInSeconds,
    default_maximal_formation_length_in_meter: Integer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct JsonInput {
    unit_types: Vec<UnitType>,
    locations: Vec<Location>,
    depots: Vec<Depot>,
    routes: Vec<Route>,
    service_trips: Vec<ServiceTrip>,
    dead_head_trips: DeadHeadTrips,
    parameters: Parameters,
}

// method to load json into these structs
fn load_json_input(path: &str) -> JsonInput {
    let mut file = File::open(path).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    serde_json::from_str(&data).unwrap()
}

// method to create locations from jsonInput
fn create_locations(json_input: &JsonInput) -> Locations {
    let mut stations: HashSet<Station> = HashSet::new();
    let mut dead_head_trips: HashMap<Station, HashMap<Station, DeadHeadTrip>> = HashMap::new();

    // add stations
    for location in &json_input.locations {
        stations.insert(Station::from(&location.id));
    }

    // add dead head trips
    for (i, origin) in json_input.dead_head_trips.indices.iter().enumerate() {
        let origin_station = Station::from(&origin);
        let mut destination_map: HashMap<Station, DeadHeadTrip> = HashMap::new();
        for (j, destination) in json_input.dead_head_trips.indices.iter().enumerate() {
            destination_map.insert(
                Station::from(&destination),
                DeadHeadTrip::new(
                    Distance::from_meter(
                        json_input.dead_head_trips.distances_in_meter[i][j] as u64,
                    ),
                    Duration::from_seconds(
                        json_input.dead_head_trips.travel_time_durations_in_seconds[i][j],
                    ),
                    StationSide::Back,  // TODO: Read this from json
                    StationSide::Front, // TODO: Read this from json
                ),
            );
        }
        dead_head_trips.insert(origin_station, destination_map);
    }

    Locations::new(stations, dead_head_trips)
}