use std::fmt;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::distance::Distance;
use crate::time::Duration;
use crate::base_types::StationSide;

use crate::utilities::CopyStr;

type Station = CopyStr<4>; // Stations are represented by String codes of length 2 to 4.



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
pub(crate) struct Locations {
    stations: HashSet<Station>,
    dead_head_trips: HashMap<Station,HashMap<Station,DeadHeadTrip>>,
}

#[derive(Hash,Eq,PartialEq,Copy,Clone)]
pub(crate) enum Location {
    Location(Station),
    Infinity // distance to Infinity is always infinity
}

struct DeadHeadTrip{
    distance: Distance,
    travel_time: Duration,
    origin_side: StationSide,
    destination_side: StationSide
}

/////////////////////////////////////////////////////////////////////
////////////////////////////// Locations ////////////////////////////
/////////////////////////////////////////////////////////////////////

// static functions
impl Locations {

    pub(crate) fn load_from_csv(path: String) -> Locations {
        let mut stations: HashSet<Station> = HashSet::new();
        let mut dead_head_trips: HashMap<Station,HashMap<Station,DeadHeadTrip>> = HashMap::new();
        let mut reader = csv::ReaderBuilder::new().delimiter(b';').from_path(path).expect("csv-file for loading locations not found");
        for result in reader.records() {
            let record = result.expect("Some recond cannot be read while reading locations");
            let first_station_code = record.get(0).unwrap();
            let second_station_code = record.get(1).unwrap();

            let travel_time_formatted = record.get(2).unwrap().split('T').last().unwrap().split('M').next().unwrap().replace("H",":");
            let travel_time = Duration::new(&travel_time_formatted);

            let distance = Distance::from_km(record.get(3).unwrap().parse().unwrap());

            let first_side = match String::from(record.get(4).unwrap()).as_str(){
                "0" => StationSide::Back,
                "1" => StationSide::Front,
                _ => panic!("StationSide is neither '0' nor '1'")
            };

            let second_side = match record.get(5).unwrap(){
                "0" => StationSide::Back,
                "1" => StationSide::Front,
                _ => panic!("StationSide is neither '0' nor '1'")
            };


            fn insert(distances: &mut HashMap<Station,HashMap<Station,DeadHeadTrip>>, origin: &Station, destination: &Station, dead_head_trip: DeadHeadTrip) {
                match distances.get_mut(origin){
                    Some(hm) => hm,
                    None => {distances.insert(origin.clone(),HashMap::new());
                             distances.get_mut(origin).unwrap()}
                }.insert(destination.clone(), dead_head_trip);
            }

            stations.insert(Station::from(first_station_code));
            stations.insert(Station::from(second_station_code));

            let first_station = Station::from(first_station_code);
            let second_station = Station::from(second_station_code);


            insert(&mut dead_head_trips,
                   &first_station,
                   &second_station,
                   DeadHeadTrip{distance,travel_time,origin_side:first_side,destination_side:second_side});

            insert(&mut dead_head_trips,
                   &second_station,
                   &first_station,
                   DeadHeadTrip{distance,travel_time,origin_side:second_side,destination_side:first_side});

            insert(&mut dead_head_trips,
                   &first_station,
                   &first_station,
                   DeadHeadTrip{distance:Distance::zero(),
                   travel_time: Duration::zero(),origin_side:StationSide::Front,destination_side:StationSide::Back});

            insert(&mut dead_head_trips,
                   &second_station,
                   &second_station,
                   DeadHeadTrip{distance:Distance::zero(), travel_time: Duration::zero(),origin_side:StationSide::Front,destination_side:StationSide::Back});


        }
        Locations{stations, dead_head_trips}

    }

}

// methods
impl Locations {
    pub(crate) fn get_all_locations(&self) -> Vec<Location> {
        self.stations.iter().map(|s| Location::of(*s)).collect()
    }

    pub(crate) fn get_location(&self, code: &str) -> Location {
        let station = Station::from(code);
        if self.stations.contains(&station) {
            Location::of(station)
        } else {
            panic!("Station code is invalid.");
        }
    }

    pub(crate) fn distance(&self, a: Location, b: Location) -> Distance {
        match self.get_dead_head_trip(a,b) {
            None => Distance::zero(),
            Some(d) => d.distance
        }
    }

    pub(crate) fn travel_time(&self, a: Location, b: Location) -> Duration {
        match self.get_dead_head_trip(a,b) {
            None => Duration::zero(),
            Some(d) => d.travel_time
        }
    }

    /// returns the StationSides of a dead-head trip. First entry is on which side the unit leaves
    /// the origin, second entry is on which side the unit enters the destination
    pub(crate) fn station_sides(&self, a: Location, b:Location) -> (StationSide, StationSide) {
        match self.get_dead_head_trip(a,b) {
            None => (StationSide::Front, StationSide::Back), // if some of the locations are Infinity, sides should not play any role
            Some(d) => (d.origin_side, d.destination_side)
        }
    }

    fn get_dead_head_trip(&self, a: Location, b: Location) -> Option<&DeadHeadTrip> {
        match a {
            Location::Infinity => None,
            Location::Location(station_a) =>
                match b {
                    Location::Infinity => None,
                    Location::Location(station_b) =>
                        Some(self.dead_head_trips.get(&station_a).unwrap().get(&station_b).unwrap())
                }
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

    fn of(station: Station) -> Location {
        Location::Location(station)
    }
}

impl Location {
    fn as_station(&self) -> Station {
        match self {
            Location::Location(s) => s.clone(),
            Location::Infinity => {panic!("Location is infinity!")},
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Location::Location(s) => write!(f, "{}", s),
            Location::Infinity => write!(f, "INFINITY!"),
        }
    }
}


