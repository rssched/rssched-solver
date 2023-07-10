use std::collections::HashMap;
use std::collections::HashSet;

use crate::base_types::{Distance, Duration, Location, LocationId, StationSide};

/// a type for storing the pair-wise distances and travel times between all stations.
/// Distances are stored as a Vec<Vec<Distance>>-matrix.
/// Travel times are stored as a Vec<Vec<Duration>>-matrix.
/// The indices in the matrix equal the indices in the station vector equal the index stored in
/// each station.
/// The distance can be obtained by the dist function which has two &Location as input and provides
/// a Distance.
/// The travel time can be obtained by the tt function which has two &Location as input and
/// provides a Duration.
///
/// Distances and travel times should satisfy the triangle-inequality. This is not asserted.
///
/// A DeadHeadMetrics instance can only be created together with the Vec<Distance> of wrapped
/// stations. Use loactions::create_locations for that. Hence, the indices should always be consistent.
pub struct Locations {
    stations: HashSet<LocationId>,
    dead_head_trips: HashMap<LocationId, HashMap<LocationId, DeadHeadTrip>>,
}

pub struct DeadHeadTrip {
    distance: Distance,
    travel_time: Duration,
    origin_side: StationSide,
    destination_side: StationSide,
}

impl DeadHeadTrip {
    pub fn new(
        distance: Distance,
        travel_time: Duration,
        origin_side: StationSide,
        destination_side: StationSide,
    ) -> DeadHeadTrip {
        DeadHeadTrip {
            distance,
            travel_time,
            origin_side,
            destination_side,
        }
    }
}

/////////////////////////////////////////////////////////////////////
////////////////////////////// Locations ////////////////////////////
/////////////////////////////////////////////////////////////////////

// static functions
impl Locations {
    pub fn new(
        stations: HashSet<LocationId>,
        dead_head_trips: HashMap<LocationId, HashMap<LocationId, DeadHeadTrip>>,
    ) -> Locations {
        Locations {
            stations,
            dead_head_trips,
        }
    }
}

// methods
impl Locations {
    // pub fn get_all_locations(&self) -> Vec<Location> {
    // let mut stations: Vec<Station> = self.stations.iter().copied().collect();
    // stations.sort();
    // stations.iter().map(|s| Location::of(*s)).collect()
    // }

    pub fn get_location(&self, location_id: LocationId) -> Location {
        if self.stations.contains(&location_id) {
            Location::of(location_id)
        } else {
            panic!("Station code is invalid.");
        }
    }

    pub fn distance(&self, a: Location, b: Location) -> Distance {
        match self.get_dead_head_trip(a, b) {
            Some(d) => d.distance,
            None => {
                if a == Location::Nowhere || b == Location::Nowhere {
                    Distance::Infinity
                } else {
                    Distance::zero()
                }
            }
        }
    }

    pub fn travel_time(&self, a: Location, b: Location) -> Duration {
        match self.get_dead_head_trip(a, b) {
            Some(d) => d.travel_time,
            None => {
                if a == Location::Nowhere || b == Location::Nowhere {
                    Duration::Infinity
                } else {
                    Duration::zero()
                }
            }
        }
    }

    /// returns the StationSides of a dead-head trip. First entry is on which side the vehicle leaves
    /// the origin, second entry is on which side the vehicle enters the destination
    pub fn station_sides(&self, a: Location, b: Location) -> (StationSide, StationSide) {
        match self.get_dead_head_trip(a, b) {
            None => (StationSide::Front, StationSide::Back), // if some of the locations are Infinity, sides should not play any role
            Some(d) => (d.origin_side, d.destination_side),
        }
    }

    fn get_dead_head_trip(&self, a: Location, b: Location) -> Option<&DeadHeadTrip> {
        match a {
            Location::Station(station_a) => match b {
                Location::Station(station_b) => Some(
                    self.dead_head_trips
                        .get(&station_a)
                        .unwrap()
                        .get(&station_b)
                        .unwrap(),
                ),
                _ => None,
            },
            _ => None,
        }
    }
}
