mod tour;
use tour::Tour;

pub(crate) mod path;
use path::Path;
use path::Segment;

pub(crate) mod objective;
use objective::ObjectiveValue;

pub(crate) mod train_formation;
use train_formation::TrainFormation;

use crate::config::Config;
use crate::network::Network;
use crate::network::nodes::Node;
use crate::units::{Units,Unit,UnitType};
use crate::distance::Distance;
use crate::base_types::{NodeId,UnitId,Cost};
use crate::time::{Time,Duration};

use std::collections::VecDeque;
use im::HashMap;

use itertools::Itertools;
use std::error::Error;
use std::cmp::Ordering;
use std::sync::Arc;


// this represents a solution to the rolling stock problem.
// It should be an immutable object. So whenever a modification is applied a copy of the
// schedule is create.
#[derive(Clone)]
pub(crate) struct Schedule {
    tours: HashMap<UnitId, Tour>,
    covered_by: HashMap<NodeId, TrainFormation>,

    // non-covered or only partially covered service nodes are stored seperately
    dummies: HashMap<UnitId, (UnitType, Tour)>,
    dummy_ids_sorted: Vec<UnitId>,
    dummy_counter: usize,

    unit_objective_info: HashMap<UnitId, ObjectiveInfo>, // for each unit we store the overhead_time and the dead_head_distance
    dummy_objective_info: HashMap<UnitId, Duration>, // for each dummy we store the overhead_time
    objective_value: ObjectiveValue,

    config: Arc<Config>,
    units: Arc<Units>,
    nw: Arc<Network>,
}

#[derive(Clone)]
struct ObjectiveInfo {
    overhead_time: Duration,
    dead_head_distance: Distance,
    maintenance_distance_violation: Distance,
    maintenance_duration_violation: Duration,
    continuous_idle_time_cost: Cost // usually this is negative (so a bonus)
}

impl ObjectiveInfo {
    fn new(unit: &Unit, tour: &Tour) -> ObjectiveInfo {
        let (maintenance_distance_violation, maintenance_duration_violation) = tour.maintenance_violation(unit);
        let continuous_idle_time_cost = tour.continuous_idle_time_cost();
        ObjectiveInfo {
            overhead_time: tour.overhead_time(),
            dead_head_distance: tour.dead_head_distance(),
            maintenance_distance_violation,
            maintenance_duration_violation,
            continuous_idle_time_cost
        }
    }
}



// methods
impl Schedule {
    pub(crate) fn tour_of(&self, unit: UnitId) -> &Tour {
        self.tours.get(&unit).unwrap_or_else(|| &self.dummies.get(&unit).unwrap_or_else(|| panic!("{} is neither real nor dummy unit", unit)).1)
    }

    pub(crate) fn covered_by(&self, node: NodeId) -> &TrainFormation {
        self.covered_by.get(&node).unwrap()
    }

    pub(crate) fn type_of(&self, unit: UnitId) -> UnitType {
        self.dummies.get(&unit).map(|tuple| tuple.0).unwrap_or_else(|| self.units.get_unit(unit).unit_type())
    }

    pub(crate) fn is_dummy(&self, unit: UnitId) -> bool {
        self.dummies.contains_key(&unit)
    }

    // pub(crate) fn total_overhead_time(&self) -> Duration {
        // self.tours.values().map(|t| t.overhead_time()).sum()
    // }

    // pub(crate) fn overhead_time_of(&self, unit: UnitId) -> Duration {
        // self.tours.get(&unit).unwrap().overhead_time()
    // }

    // pub(crate) fn total_dummy_overhead_time(&self) -> Duration {
        // self.dummies.values().map(|tuple| tuple.1.overhead_time()).sum()
    // }

    // pub(crate) fn total_distance(&self) -> Distance {
        // self.tours.values().map(|t| t.distance()).sum()
    // }

    // pub(crate) fn total_dead_head_distance(&self) -> Distance {
        // self.tours.values().map(|t| t.dead_head_distance()).sum()
    // }

    pub(crate) fn number_of_dummy_units(&self) -> usize {
        self.dummies.len()
    }

    pub(crate) fn objective_value(&self) -> ObjectiveValue {
        self.objective_value
    }

    // returns the first (seen from head to tail) dummy_unit that covers the node.
    // If node is fully-covered by real units, None is returned.
    fn get_dummy_cover_of(&self, node: NodeId) -> Option<UnitId> {
        self.covered_by.get(&node).unwrap().iter().find(|u| self.dummies.contains_key(u))
    }

    // pub(crate) fn uncovered_nodes(&self) -> impl Iterator<Item = (NodeId,UnitId)> + '_ {
        // self.dummy_iter().flat_map(|u| self.tour_of(u).nodes_iter().map(move |n| (*n,u)))
    // }

    pub(crate) fn dummy_iter(&self) -> impl Iterator<Item = UnitId> + '_ {
        self.dummy_ids_sorted.iter().copied()
    }

    /// returns all unit ids of real units (sorted)
    pub(crate) fn real_units_iter(&self) -> impl Iterator<Item = UnitId> + '_ {
        self.units.iter()
    }

    pub(crate) fn uncovered_successors(&self, node: NodeId) -> impl Iterator<Item = (NodeId,UnitId)> + '_ {
        self.nw.all_successors(node).filter_map(|n| self.get_dummy_cover_of(n).map(|u| (n,u)))
    }

    /// Simulates inserting the node_sequence into the tour of unit. Return all nodes (as a Path) that would
    /// have been removed from the tour.
    pub(crate) fn conflict(&self, segment: Segment, receiver: UnitId) -> Result<Path,String> {
        let tour: Tour = self.tour_of(receiver).clone();
        let result = tour.conflict(segment)?;
        Ok(result)
    }

    pub(crate) fn conflict_single_node(&self, node: NodeId, receiver: UnitId) -> Result<Path, String> {
        self.conflict(Segment::new(node,node),receiver)
    }

    // pub(crate) fn conflict_all(&self, dummy_provider: UnitId, receiver: UnitId) -> Result<Path, String> {
        // let tour = &self.dummies.get(&dummy_provider).expect("Can only assign_all if provider is a dummy-unit.").1;
        // self.conflict(Segment::new(tour.first_node(), tour.last_node()), receiver)
    // }

    /// Reassigns a path (given by a segment and a provider) to the tour of receiver.
    /// Aborts if there are any conflicts.
    pub(crate) fn reassign(&self, segment: Segment, provider: UnitId, receiver: UnitId) -> Result<Schedule, String> {
        if !self.conflict(segment, receiver)?.is_empty() {
            return Err(String::from("There are conflcits. Abort reassign()!"));
        }
        self.override_reassign(segment, provider, receiver).map(|tuple| tuple.0)
    }

    /// Reassigns a single node of provider to the tour of receiver.
    /// Aborts if there are any conflicts.
    // pub(crate) fn reassign_single_node(&self, node: NodeId, provider: UnitId, receiver: UnitId) -> Result<Schedule,String> {
        // self.reassign(Segment::new(node, node), provider, receiver)
    // }

    /// Reassign the complete tour of the provider (must be dummy-unit) to the receiver.
    /// Aborts if there are any conflicts.
    pub(crate) fn reassign_all(&self, dummy_provider: UnitId, receiver: UnitId) -> Result<Schedule, String> {
        let tour = &self.dummies.get(&dummy_provider).expect("Can only assign_all if provider is a dummy-unit.").1;
        self.reassign(Segment::new(tour.first_node(), tour.last_node()), dummy_provider, receiver)
    }

    /// Reassigns a single node of provider to the tour of receiver.
    /// Conflicts are removed from the tour.
    // pub(crate) fn override_reassign_single_node(&self, node: NodeId, provider: UnitId, receiver: UnitId) -> Result<(Schedule, Option<UnitId>),String> {
        // self.override_reassign(Segment::new(node, node), provider, receiver)
    // }

    /// Reassign the complete tour of the provider (must be dummy-unit) to the receiver.
    /// Conflicts are removed from the tour.
    pub(crate) fn override_reassign_all(&self, dummy_provider: UnitId, receiver: UnitId) -> Result<(Schedule, Option<UnitId>), String> {
        let tour = &self.dummies.get(&dummy_provider).expect("Can only assign_all if provider is a dummy-unit.").1;
        self.override_reassign(Segment::new(tour.first_node(), tour.last_node()), dummy_provider, receiver)
    }

    /// Tries to insert all nodes of provider's segment into receiver's tour.
    /// Nodes that causes conflcits are rejected and stay in provider's tour.
    /// Nodes that do not cause a conflict are reassigned to the receiver.
    pub(crate) fn fit_reassign(&self, segment: Segment, provider: UnitId, receiver: UnitId) -> Result<Schedule,String> {

        // do lazy clones of schedule:
        let mut tours = self.tours.clone();
        let mut covered_by = self.covered_by.clone();
        let mut dummies = self.dummies.clone();
        let mut dummy_ids_sorted = self.dummy_ids_sorted.clone();
        let mut unit_objective_info = self.unit_objective_info.clone();
        let mut dummy_objective_info = self.dummy_objective_info.clone();

        let tour_provider = self.tour_of(provider);
        let tour_receiver = self.tour_of(receiver);

        let mut new_tour_provider = tour_provider.clone();
        let mut new_tour_receiver = tour_receiver.clone();

        let mut remaining_path = tour_provider.sub_path(segment)?;

        let mut moved_nodes = Vec::new();


        // go through the path that should be inserted without causing conflcits.
        // As dead_head_trips might be longer than service trips we do not iterate over all nodes
        // individually but instead cut the path into maximal segments that could be reassigned.
        //
        // Hence we iteratively consider the first node of the remaining_path as the start of a
        // segment and take the biggest segment that can be reassigned.
        // Afterwards this segment is removed

        while !remaining_path.is_empty() {
            let sub_segment_start = remaining_path.first();
            let (end_pos, sub_segment_end) = match new_tour_receiver.earliest_not_reaching_node(sub_segment_start) {
                None => (remaining_path.len() - 1, remaining_path.last()),
                Some(pos) => {
                    // the segment can only be inserted before the blocker
                    let blocker = new_tour_receiver.nth_node(pos);
                    // consider all nodes that arrive before the departure of the blocker
                    // test all of them if they can reach the blocker.
                    // test all of them if this segment could be removed.
                    // take the latest node of those.
                    // If empty this segment will fail, so return path.first()
                    remaining_path.iter().enumerate()
                        .map_while(|(i,&n)| if self.nw.node(n).end_time() > self.nw.node(blocker).start_time() {None} else {Some((i,n))})
                        .filter(|(_,n)| self.nw.can_reach(*n, blocker))
                        .filter(|(_,n)| new_tour_provider.removable(Segment::new(sub_segment_start, *n)))
                        .last().unwrap_or((0,remaining_path.first()))
                }
            };

            let mut node_sequence = remaining_path.consume();
            remaining_path = Path::new(node_sequence.split_off(end_pos+1), self.nw.clone());
            let sub_segment = Segment::new(sub_segment_start, sub_segment_end);
            let remove_result = new_tour_provider.remove(sub_segment);

            if remove_result.is_err() {
                continue;
            }

            let (new_tour_provider_candidate, path_for_insertion) = remove_result.unwrap();

            // test if inserting sub_segment would cause any conflicts (or fail for other reasons
            if new_tour_receiver.conflict(sub_segment).map(|c| !c.is_empty()).unwrap_or(true) {
                continue;
            }
            let insert_result = new_tour_receiver.insert(path_for_insertion);

            if let Ok(receiver) = insert_result {
                new_tour_provider = new_tour_provider_candidate;
                new_tour_receiver = receiver;
                moved_nodes.extend(node_sequence);
            }

        }

        // update reduced tour of the provider
        if new_tour_provider.len() > 0 {
            if self.is_dummy(provider) {
                dummy_objective_info.insert(provider, new_tour_provider.overhead_time());
                dummies.insert(provider, (self.type_of(provider),new_tour_provider));
            } else {
                unit_objective_info.insert(provider, ObjectiveInfo::new(self.units.get_unit(provider), &new_tour_provider));
                tours.insert(provider, new_tour_provider);
            }
        } else {
            dummies.remove(&provider); // old_dummy_tour is completely removed
            dummy_ids_sorted.remove(dummy_ids_sorted.binary_search(&provider).unwrap());
            dummy_objective_info.remove(&provider);
        }

        // update extended tour of the receiver
        if self.is_dummy(receiver) {
            dummy_objective_info.insert(receiver, new_tour_receiver.overhead_time());
            dummies.insert(receiver, (self.type_of(receiver), new_tour_receiver));
        } else {
            unit_objective_info.insert(receiver, ObjectiveInfo::new(self.units.get_unit(receiver), &new_tour_receiver));
            tours.insert(receiver, new_tour_receiver);
        }

        // update covered_by:
        for node in moved_nodes.iter() {
            let new_formation = covered_by.get(node).unwrap().replace(provider, receiver);
            covered_by.insert(*node, new_formation);
        }


        let objective_value = Schedule::sum_up_objective_info(&unit_objective_info, &dummy_objective_info, self.config.clone(), &self.units);


        Ok(Schedule{tours,
            covered_by,
            dummies,
            dummy_ids_sorted,
            dummy_counter: self.dummy_counter,
            unit_objective_info,
            dummy_objective_info,
            objective_value,
            config:self.config.clone(),
            units:self.units.clone(),
            nw:self.nw.clone()})
    }

    pub(crate) fn fit_reassign_all(&self, provider: UnitId, receiver: UnitId) -> Result<Schedule,String> {
        let provider_tour = self.tour_of(provider);
        self.fit_reassign(Segment::new(provider_tour.first_node(), provider_tour.last_node()), provider, receiver)
    }


    /// Remove segment from provider's tour and inserts the nodes into the tour of receiver unit.
    /// All conflicting nodes are removed from the tour and in the case that there are conflcits
    /// a new dummy tour is created.
    /// If path ends with an endnode it is replaces the old endpoint. (Path is suffix of the tour.)
    /// Otherwise the path must reach the old endnode.
    pub(crate) fn override_reassign(&self, segment: Segment, provider: UnitId, receiver: UnitId) -> Result<(Schedule, Option<UnitId>),String> {

        // do lazy clones of schedule:
        let mut tours = self.tours.clone();
        let mut covered_by = self.covered_by.clone();
        let mut dummies = self.dummies.clone();
        let mut dummy_ids_sorted = self.dummy_ids_sorted.clone();
        let mut dummy_counter = self.dummy_counter;
        let mut unit_objective_info = self.unit_objective_info.clone();
        let mut dummy_objective_info = self.dummy_objective_info.clone();

        let tour_provider = self.tour_of(provider);
        let tour_receiver = self.tour_of(receiver);

        // remove segment for provider
        let (shrinked_tour_provider, path) = tour_provider.remove(segment)?;


        // update covered_by:
        for node in path.iter() {
            let new_formation = covered_by.get(node).unwrap().replace(provider, receiver);
            covered_by.insert(*node, new_formation);
        }

        // insert path into tour
        let replaced_path = tour_receiver.conflict(segment)?;
        let new_tour_receiver = tour_receiver.insert(path)?;




        // update shrinked tour of the provider
        if shrinked_tour_provider.len() > 0 {
            if self.is_dummy(provider) {
                dummy_objective_info.insert(provider, shrinked_tour_provider.overhead_time());
                dummies.insert(provider, (self.type_of(provider), shrinked_tour_provider));
            } else {
                unit_objective_info.insert(provider, ObjectiveInfo::new(self.units.get_unit(provider), &shrinked_tour_provider));
                tours.insert(provider, shrinked_tour_provider);
            }
        } else {
            dummies.remove(&provider); // old_dummy_tour is completely removed
            dummy_ids_sorted.remove(dummy_ids_sorted.binary_search(&provider).unwrap());
            dummy_objective_info.remove(&provider);
        }

        // update extended tour of the receiver
        if self.is_dummy(receiver) {
            dummy_objective_info.insert(receiver, new_tour_receiver.overhead_time());
            dummies.insert(receiver, (self.type_of(receiver), new_tour_receiver));
        } else {
            unit_objective_info.insert(receiver, ObjectiveInfo::new(self.units.get_unit(receiver), &new_tour_receiver));
            tours.insert(receiver, new_tour_receiver);
        }

        let mut new_dummy_opt = None;
        // insert new dummy tour consisting of conflicting nodes removed from receiver's tour
        if !replaced_path.is_empty() {

            let new_dummy = UnitId::from(format!("dummy{:05}", dummy_counter).as_str());

            new_dummy_opt = Some(new_dummy);

            for node in replaced_path.iter() {
                let new_formation = covered_by.get(node).unwrap().replace(receiver, new_dummy);
                covered_by.insert(*node, new_formation);
            }

            let new_dummy_type = self.type_of(receiver);
            let new_dummy_tour = Tour::new_dummy_by_path(new_dummy_type, replaced_path, self.config.clone(), self.nw.clone());

            dummy_objective_info.insert(new_dummy, new_dummy_tour.overhead_time());
            dummies.insert(new_dummy, (new_dummy_type, new_dummy_tour));
            dummy_ids_sorted.insert(dummy_ids_sorted.binary_search(&new_dummy).unwrap_or_else(|e| e), new_dummy);
            // dummy_ids_sorted.push(new_dummy);
            // dummy_ids_sorted.sort();

            dummy_counter += 1;
        }


        let objective_value = Schedule::sum_up_objective_info(&unit_objective_info, &dummy_objective_info, self.config.clone(), &self.units);


        Ok((Schedule{tours,
            covered_by,
            dummies,
            dummy_ids_sorted,
            dummy_counter,
            unit_objective_info,
            dummy_objective_info,
            objective_value,
            config:self.config.clone(),
            units:self.units.clone(),
            nw:self.nw.clone()},
            new_dummy_opt))
    }


    pub(crate) fn write_to_csv(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut writer = csv::WriterBuilder::new().delimiter(b';').from_path(path)?;
        writer.write_record(&["fahrzeuggruppeId","sortierZeit","typ","bpAb","bpAn","kundenfahrtId","endpunktId","wartungsfensterId"])?;
        for unit in self.units.iter() {
            let tour = self.tours.get(&unit).unwrap();
            for (prev_node_id, node_id) in tour.nodes_iter().tuple_windows() {
                let node = self.nw.node(*node_id);

                let prev_node = self.nw.node(*prev_node_id);

                let fahrzeuggruppen_id = format!("{}",unit);



                if prev_node.end_location() != node.start_location() {
                    // add dead_head_trip
                    let dhd_ab = format!("{}", prev_node.end_location());
                    let dhd_an = format!("{}", node.start_location());
                    let dhd_sortier_zeit = prev_node.end_time().as_iso();
                    let empty = String::from("");
                    writer.write_record(&[fahrzeuggruppen_id.clone(),dhd_sortier_zeit,String::from("BETRIEBSFAHRT"),dhd_ab,dhd_an,empty.clone(),empty.clone(),empty])?;
                }



                let sortier_zeit = node.start_time().as_iso();
                let typ = String::from(match node {
                    Node::Service(_) => "KUNDENFAHRT",
                    Node::Maintenance(_) => "WARTUNG",
                    Node::End(_) => {"ENDPUNKT"},
                    _ => ""
                });
                let (bp_ab, bp_an) = match node {
                    Node::End(_) => (String::from(""), format!("{}", node.start_location())),
                    _ => (format!("{}", node.start_location()), format!("{}", node.end_location()))
                };

                let long_id = format!("{}", node.id());
                let id: &str = long_id.split(':').collect::<Vec<_>>().get(1).unwrap(); // remove the "ST:", "MS:", "EP:"
                let kundenfahrt_id = String::from(match node {
                    Node::Service(_) => id,
                    _ => ""
                });
                let endpunkt_id = String::from(match node {
                    Node::End(_) => id,
                    _ => ""
                });
                let wartungsfenster_id = String::from(match node {
                    Node::Maintenance(_) => id,
                    _ => ""
                });
                writer.write_record(&[fahrzeuggruppen_id,sortier_zeit,typ,bp_ab,bp_an,kundenfahrt_id,endpunkt_id,wartungsfenster_id])?;
            }
        }

        Ok(())

    }


    pub(crate) fn print_long(&self) {
        println!("** schedule with {} tours and {} dummy-tours:", self.tours.len(), self.dummies.len());
        for unit in self.units.iter() {
            print!("     {}: ", unit);
            self.tours.get(&unit).unwrap().print();
        }
        for dummy in self.dummy_iter() {
            print!("     {}: ", dummy);
            self.dummies.get(&dummy).unwrap().1.print();
        }
    }

    pub(crate) fn print(&self) {

        for unit in self.units.iter() {
            println!("{}: {}", unit, self.tours.get(&unit).unwrap());
        }
        for dummy in self.dummy_iter() {
            println!("{}: {}", dummy, self.dummies.get(&dummy).unwrap().1);
        }
    }

    pub(crate) fn cmp_objective_values(&self, other: &Self) -> Ordering {
        self.objective_value.cmp(&other.objective_value)
    }

}


impl Ord for Schedule {
    fn cmp(&self, other: &Self) -> Ordering {
        // first compare objective
        self.objective_value.cmp(&other.objective_value)
            // then compare real unit tours
            .then(match self.units.iter().map(|u| self.tour_of(u).cmp(other.tour_of(u))).find(|ord| *ord != Ordering::Equal) {
                Some(other) => other,
                None => {
                    // finally compare dummy_tours. For this first sort the dummy tours and
                    // then compare from small to long.
                    let mut dummy_tours: Vec<_> = self.dummies.values().collect();
                    dummy_tours.sort_by(|tuple, other_tuple| tuple.1.cmp(&other_tuple.1));
                    let mut other_dummy_tours: Vec<_> = other.dummies.values().collect();
                    other_dummy_tours.sort_by(|tuple, other_tuple| tuple.1.cmp(&other_tuple.1));
                    match dummy_tours.iter().zip(other_dummy_tours.iter())
                            .map(|(&tuple, &other_tuple)| tuple.1.cmp(&other_tuple.1)).find(|ord| *ord != Ordering::Equal) {
                        Some(other) => other,
                        None => Ordering::Equal
                    }
                }
            })
    }
}

impl PartialOrd for Schedule {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Schedule {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Schedule {}

// impl Hash for Schedule {
    // fn hash<H: Hasher>(&self, state: &mut H) {
        // let covered_by: Vec<_> = self.nw.all_nodes().flat_map(|n| self.covered_by(n).iter()).collect();
        // covered_by.hash(state)
    // }
// }


// static functions
impl Schedule {
    pub(crate) fn initialize(config: Arc<Config>, units: Arc<Units>, nw: Arc<Network>) -> Schedule {

        let mut tours: HashMap<UnitId, Tour> = HashMap::new();
        let mut covered_by: HashMap<NodeId, TrainFormation> = HashMap::new();

        // create trivial tours from start_point directly to end point
        // end_ponints are picked greedily from earliest to latest (unit_type must fit)
        let mut end_nodes: VecDeque<NodeId> = nw.end_nodes().collect();
        end_nodes.make_contiguous().sort_by(|&e1,&e2| nw.node(e1).start_time().cmp(&nw.node(e2).start_time()));

        for unit_id in units.iter() {
            let unit = units.get_unit(unit_id);
            let start_node = nw.start_node_of(unit_id);
            let pos = end_nodes.iter().position(|&e| nw.node(e).unit_type() == unit.unit_type() && nw.can_reach(start_node, e)).unwrap_or_else(|| panic!("No suitable end_node found for start_node: {}", start_node));
            let end_node = end_nodes.remove(pos).unwrap();

            tours.insert(unit_id, Tour::new(unit.unit_type(), vec!(start_node, end_node), config.clone(), nw.clone()).unwrap());

            covered_by.insert(start_node, TrainFormation::new(vec!(unit_id), units.clone()));
            covered_by.insert(end_node, TrainFormation::new(vec!(unit_id), units.clone()));
        }
        assert!(end_nodes.is_empty(), "There are more end_nodes than units!");

        // all service- and maintanence nodes are non covered. We create dummy_units to coverer
        // them. Each dummy_unit has a Tour of exactly one node.
        let mut dummies = HashMap::new();
        let mut dummy_counter = 0;

        for node in nw.service_nodes().chain(nw.maintenance_nodes()) {
            let mut formation = Vec::new();
            for t in nw.node(node).demand().get_valid_types() {
                let trivial_tour = Tour::new_dummy(t, vec!(node), config.clone(), nw.clone()).unwrap();
                let new_dummy_id = UnitId::from(format!("dummy{:05}", dummy_counter).as_str());

                dummies.insert(new_dummy_id,(t,trivial_tour));

                formation.push(new_dummy_id);
                dummy_counter += 1;
            }
            covered_by.insert(node, TrainFormation::new(formation, units.clone()));
        }
        let mut dummy_ids_sorted: Vec<UnitId> = dummies.keys().copied().collect();
        dummy_ids_sorted.sort();



        // compute objective_value / unit_objective_info
        let (unit_objective_info, dummy_objective_info, objective_value) = Schedule::compute_objective_value(&tours, &dummies, config.clone(), units.clone());

        Schedule{tours,
                 covered_by,
                 dummies,
                 dummy_ids_sorted,
                 dummy_counter,
                 unit_objective_info,
                 dummy_objective_info,
                 objective_value,
                 config,
                 units,
                 nw}
    }

    fn compute_objective_value(tours: &HashMap<UnitId, Tour>, dummies: &HashMap<UnitId, (UnitType, Tour)>, config: Arc<Config>, units: Arc<Units>) -> (HashMap<UnitId, ObjectiveInfo>, HashMap<UnitId, Duration>, ObjectiveValue) {
        // compute objective_value / unit_objective_info
        let mut unit_objective_info: HashMap<UnitId, ObjectiveInfo> = HashMap::new();
        let mut dummy_objective_info: HashMap<UnitId, Duration> = HashMap::new();
        for unit in tours.keys() {
            let tour = tours.get(unit).unwrap();
            unit_objective_info.insert(*unit, ObjectiveInfo::new(units.get_unit(*unit), tour));
        }
        for dummy in dummies.keys() {
            dummy_objective_info.insert(*dummy, Duration::zero());
        }

        let objective_value = Schedule::sum_up_objective_info(&unit_objective_info, &dummy_objective_info, config, &units);


        (unit_objective_info, dummy_objective_info, objective_value)

    }

    fn sum_up_objective_info(unit_objective_info: &HashMap<UnitId, ObjectiveInfo>, dummy_objective_info: &HashMap<UnitId, Duration>, config: Arc<Config>, units: &Arc<Units>) -> ObjectiveValue {
        let overhead_time = unit_objective_info.values().map(|info| info.overhead_time).sum();
        let number_of_dummy_units = dummy_objective_info.len();
        let dummy_overhead_time: Duration = dummy_objective_info.values().copied().sum();
        let maintenance_distance_violation = unit_objective_info.values().map(|info| info.maintenance_distance_violation).sum();
        let maintenance_duration_violation = unit_objective_info.values().map(|info| info.maintenance_duration_violation).sum();
        let dead_head_distance = unit_objective_info.values().map(|info| info.dead_head_distance).sum();

        // sum up in the deterministic ordering given by units.iter():
        let continuous_idle_time_cost = units.iter().map(|u| unit_objective_info.get(&u).unwrap().continuous_idle_time_cost).sum();

        ObjectiveValue::new(overhead_time,
                            number_of_dummy_units,
                            dummy_overhead_time,
                            maintenance_distance_violation,
                            maintenance_duration_violation,
                            dead_head_distance,
                            continuous_idle_time_cost,
                            config)
    }


    pub(crate) fn load_from_csv(path: &str, config: Arc<Config>, units: Arc<Units>, nw: Arc<Network>) -> Schedule {

        let mut tour_nodes: HashMap<UnitId, Vec<NodeId>> = HashMap::new();
        for unit in units.iter() {
            tour_nodes.insert(unit, Vec::new());
        }

        let mut covering: HashMap<NodeId, Vec<UnitId>> = HashMap::new();
        for node in nw.all_nodes() {
            covering.insert(node, Vec::new());
        }

        let mut reader = csv::ReaderBuilder::new().delimiter(b';').from_path(path).expect("csv-file for loading schedule not found");
        for result in reader.records() {
            let record = result.expect("Some recond cannot be read while reading service_trips");
            let unit = UnitId::from(record.get(0).unwrap());
            let _sort_time = Time::new(record.get(1).unwrap());
            let activity_type = record.get(2).unwrap();
            // let _start_location = loc.get_location(record.get(3).unwrap());
            // let _end_location = loc.get_location(record.get(4).unwrap());

            let service_trip_id = record.get(5).unwrap();
            let end_point_id = record.get(6).unwrap();
            let maintenance_shift_id = record.get(7).unwrap();


            // asserts:
            assert!(units.contains(unit), "ReadError: unit_id is invalid.");

            match activity_type {
                "KUNDENFAHRT" => {
                    let node = NodeId::from(&format!("ST:{}", service_trip_id));
                    tour_nodes.get_mut(&unit).unwrap().push(node);
                    covering.get_mut(&node).unwrap().push(unit);
                },
                "ENDPUNKT" => {
                    let node = NodeId::from(&format!("EN:{}", end_point_id));
                    tour_nodes.get_mut(&unit).unwrap().push(node);
                    covering.get_mut(&node).unwrap().push(unit);

                },
                "WARTUNG" => {
                    let node = NodeId::from(&format!("MS:{}", maintenance_shift_id));
                    tour_nodes.get_mut(&unit).unwrap().push(node);
                    covering.get_mut(&node).unwrap().push(unit);
                },
                _ => {}
            };
        }

        let mut tours: HashMap<UnitId, Tour> = HashMap::new();
        for unit in units.iter() {
            let mut nodes = tour_nodes.remove(&unit).unwrap();
            nodes.push(nw.start_node_of(unit));
            nodes.sort_by(|n1, n2| nw.node(*n1).cmp_start_time(nw.node(*n2)));
            let tour = match Tour::new_allow_invalid(units.get_unit(unit).unit_type(), nodes, config.clone(), nw.clone()) {
                Err((tour, error_msg)) => {
                    println!("{}", error_msg);
                    tour
                },
                Ok(tour) => tour
            };

            tours.insert(unit, tour);
        }

        let mut covered_by: HashMap<NodeId, TrainFormation> = HashMap::new();
        let mut dummies: HashMap<UnitId, (UnitType, Tour)> = HashMap::new();
        let mut dummy_counter = 0;
        for node in nw.service_nodes().chain(nw.maintenance_nodes()) {
            let mut formation = covering.remove(&node).unwrap();
            let types = formation.iter().map(|u| units.get_unit(*u).unit_type()).collect();
            for t in nw.node(node).demand().get_missing_types(&types) {
                let trivial_tour = Tour::new_dummy(t, vec!(node), config.clone(), nw.clone()).unwrap();
                let new_dummy_id = UnitId::from(format!("dummy{:05}", dummy_counter).as_str());

                dummies.insert(new_dummy_id,(t,trivial_tour));

                formation.push(new_dummy_id);
                dummy_counter += 1;
            }
            covered_by.insert(node, TrainFormation::new(formation, units.clone()));
        }

        for node in nw.end_nodes() {
            let formation = covering.remove(&node).unwrap();
            covered_by.insert(node, TrainFormation::new(formation, units.clone()));
        }

        for unit in units.iter() {
            covered_by.insert(nw.start_node_of(unit), TrainFormation::new(vec!(unit), units.clone()));
        }

        let mut dummy_ids_sorted: Vec<UnitId> = dummies.keys().copied().collect();
        dummy_ids_sorted.sort();

        // compute objective_value / unit_objective_info
        let (unit_objective_info, dummy_objective_info, objective_value) = Schedule::compute_objective_value(&tours, &dummies, config.clone(), units.clone());

        Schedule{tours,
                 covered_by,
                 dummies,
                 dummy_ids_sorted,
                 dummy_counter,
                 unit_objective_info,
                 dummy_objective_info,
                 objective_value,
                 config,
                 units,
                 nw}

    }
}
