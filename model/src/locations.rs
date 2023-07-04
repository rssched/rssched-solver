use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

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

    pub fn load_from_csv(path: &str) -> Locations {
        let mut stations: HashSet<LocationId> = HashSet::new();
        let mut dead_head_trips: HashMap<LocationId, HashMap<LocationId, DeadHeadTrip>> =
            HashMap::new();
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(path)
            .expect("csv-file for loading locations not found");
        for result in reader.records() {
            let record = result.expect("Some recond cannot be read while reading locations");
            let first_station_code = record.get(0).unwrap();
            let second_station_code = record.get(1).unwrap();

            let distance = Distance::from_km(record.get(2).unwrap().parse().unwrap());

            let travel_time_formatted = record
                .get(3)
                .unwrap()
                .split('T')
                .last()
                .unwrap()
                .split('M')
                .next()
                .unwrap()
                .replace('H', ":");
            let travel_time = Duration::new(&travel_time_formatted);

            let first_side = StationSide::from(record.get(4).unwrap());

            let second_side = StationSide::from(record.get(5).unwrap());

            fn insert(
                distances: &mut HashMap<LocationId, HashMap<LocationId, DeadHeadTrip>>,
                origin: &LocationId,
                destination: &LocationId,
                dead_head_trip: DeadHeadTrip,
            ) {
                match distances.get_mut(origin) {
                    Some(hm) => hm,
                    None => {
                        distances.insert(*origin, HashMap::new());
                        distances.get_mut(origin).unwrap()
                    }
                }
                .insert(*destination, dead_head_trip);
            }

            stations.insert(LocationId::from(first_station_code));
            stations.insert(LocationId::from(second_station_code));

            let first_station = LocationId::from(first_station_code);
            let second_station = LocationId::from(second_station_code);

            insert(
                &mut dead_head_trips,
                &first_station,
                &second_station,
                DeadHeadTrip {
                    distance,
                    travel_time,
                    origin_side: first_side,
                    destination_side: second_side,
                },
            );

            insert(
                &mut dead_head_trips,
                &second_station,
                &first_station,
                DeadHeadTrip {
                    distance,
                    travel_time,
                    origin_side: second_side,
                    destination_side: first_side,
                },
            );

            insert(
                &mut dead_head_trips,
                &first_station,
                &first_station,
                DeadHeadTrip {
                    distance: Distance::zero(),
                    travel_time: Duration::zero(),
                    origin_side: StationSide::Front,
                    destination_side: StationSide::Back,
                },
            );

            insert(
                &mut dead_head_trips,
                &second_station,
                &second_station,
                DeadHeadTrip {
                    distance: Distance::zero(),
                    travel_time: Duration::zero(),
                    origin_side: StationSide::Front,
                    destination_side: StationSide::Back,
                },
            );
        }
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

/////////////////////////////////////////////////////////////////////
////////////////////////////// Location /////////////////////////////
/////////////////////////////////////////////////////////////////////

impl Location {
    // fn from(station_code: &str) -> Location {
    // Location::Location(Station::from(station_code))
    // }

    fn of(station: LocationId) -> Location {
        Location::Station(station)
    }
}

// impl Location {
// fn as_station(&self) -> Station {
// match self {
// Location::Station(s) => *s,
// _ => {panic!("Location is NOWHERE or EVERYWHERE!")},
// }
// }
// }

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Location::Station(s) => write!(f, "{}", s),
            Location::Nowhere => write!(f, "NOWHERE!"),
            // Location::Everywhere => write!(f, "EVERYWHERE!")
        }
    }
}
