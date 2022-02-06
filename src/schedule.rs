mod tour;
use tour::Tour;

use crate::network::Network;
use crate::vehicle::Vehicle;
use crate::locations::Locations;

use std::fmt;

pub(crate) struct Schedule<'a> {
    tours: Vec<Tour<'a>>,
}


impl<'a> Schedule<'a> {
    pub(crate) fn initialize(vehicles: &'a Vec<Vehicle>, network: &'a Network<'a>) -> Schedule<'a> {
        let (start_nodes, end_nodes) = network.terminal_nodes();

        let mut tours : Vec<Tour<'a>> = Vec::with_capacity(vehicles.len());
        for (i, vehicle) in vehicles.iter().enumerate() {
            tours.push(Tour::new(vehicle, vec!(&start_nodes[i], &end_nodes[i])));
        }

        Schedule{tours}
    }

    pub(crate) fn print(&self, locations: &Locations) {
        println!("** Schedule with {} tours:", self.tours.len());
        for tour in self.tours.iter() {
            println!("\t{} of length {} and travel time: {}", tour, tour.length(locations), tour.travel_time(locations));
        }
    }
}


impl<'a> fmt::Display for Schedule<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "** Schedule with {} tours:", self.tours.len())?;
        for tour in self.tours.iter() {
            writeln!(f, "\t{}", tour)?;
        }
        Ok(())
    }

}
