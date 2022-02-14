use std::fmt;
use crate::distance::Distance;
use crate::time::{Time, Duration};
use crate::locations::Locations;
use crate::network::Network;
use crate::base_types::{NodeId,UnitId};

use itertools::Itertools;

type Position = usize; // the position within the tour from 0 to nodes.len()-1

pub(crate) struct Tour<'a> {
    unit: UnitId,
    nodes: Vec<NodeId>, // nodes will always be sorted by start_time
    loc: &'a Locations,
    nw: &'a Network<'a>,
}

impl<'a> Tour<'a> {
    pub(crate) fn last_node(&self) -> NodeId {
        *self.nodes.last().unwrap()
    }

    pub(crate) fn length(&self) -> Distance {
        let service_length: Distance = self.nodes.iter().map(|&n| self.nw.node(n).length()).sum();

        let dead_head_length = self.nodes.iter().tuple_windows().map(
            |(&a,&b)| self.loc.distance(self.nw.node(a).end_location(),self.nw.node(b).start_location())).sum();
        service_length + dead_head_length
    }

    pub(crate) fn travel_time(&self) -> Duration {
        let service_tt: Duration = self.nodes.iter().map(|&n| self.nw.node(n).travel_time()).sum();
        let dead_head_tt = self.nodes.iter().tuple_windows().map(
            |(&a,&b)| self.loc.travel_time(self.nw.node(a).end_location(), self.nw.node(b).start_location())).sum();
        service_tt + dead_head_tt
    }
    /// inserts the provided node sequence on the correct position (time-wise). The sequence will
    /// stay uninterrupted. All removed nodes (due to time-clashes) are returned.
    /// Assumes that provided node sequence is feasible.
    /// Panics if sequence is not reachable from the start node, and if end_node cannot be reached,
    /// sequence must itself end with a end_node
    pub(super) fn insert(&mut self, node_sequence: Vec<NodeId>) -> Vec<NodeId> {
        let first = node_sequence[0];
        let last = node_sequence[node_sequence.len()-1];

        let start_pos = self.latest_node_reaching(first).expect(format!("Unit {}, cannot reach node {}", self.unit, first).as_str());
        let end_pos = self.earliest_node_reached_by(last).expect(format!("Cannot insert sequence to path of unit {}, as the end_point cannot be reached!", self.unit).as_str());

        // remove all elements strictly between start_pos and end_pos and replace them by
        // node_sequence. Removed nodes are returned.
        self.nodes.splice(start_pos+1..end_pos,node_sequence).collect()
    }

    fn latest_node_reaching(&self, node: NodeId) -> Option<Position>{
        if !self.nw.can_reach(self.nodes[0], node) {
            None
        } else {

            let candidate = self.latest_arrival_before(self.nw.node(node).start_time(), 0, self.nodes.len());
            match candidate {
                None => None,
                Some(p) => {
                    let mut pos = p;
                    while !self.nw.can_reach(self.nodes[pos],node) {
                        pos -= 1;
                    }
                    Some(pos)
                }
            }
        }
    }

    fn latest_arrival_before(&self, time: Time, left: Position, right: Position) -> Option<Position> {
        if left+1 == right {
            if self.nw.node(self.nodes[left]).end_time() <= time { Some(left) } else { None }
        } else {
            let mid = left + (right - left) / 2;
            if self.nw.node(self.nodes[mid]).end_time() <= time {
                self.latest_arrival_before(time, mid, right)
            } else {
                self.latest_arrival_before(time, left, mid)
            }
        }
    }

    fn earliest_node_reached_by(&self, node: NodeId) -> Option<Position>{
        if !self.nw.can_reach(node, *self.nodes.last().unwrap()) {
            None
        } else {

            let candidate = self.earliest_departure_after(self.nw.node(node).end_time(), 0, self.nodes.len());
            match candidate {
                None => None,
                Some(p) => {
                    let mut pos = p;
                    while !self.nw.can_reach(node, self.nodes[pos]) {
                        pos += 1;
                    }
                    Some(pos)
                }
            }
        }
    }

    fn earliest_departure_after(&self, time: Time, left: Position, right: Position) -> Option<Position> {
        if left+1 == right {
            if self.nw.node(self.nodes[left]).start_time() >= time { Some(left) } else { None }
        } else {
            let mid = left + (right - left - 1) / 2;
            if self.nw.node(self.nodes[mid]).start_time() >= time {
                self.earliest_departure_after(time, left, mid+1)
            } else {
                self.earliest_departure_after(time, mid+1, right)
            }
        }
    }

    pub(crate) fn print(&self) {
        println!("tour with {} nodes of length {} and travel time {}:", self.nodes.len(), self.length(), self.travel_time());
        for node in self.nodes.iter() {
            println!("\t\t* {}", node);
        }
    }
}

impl<'a> Tour<'a> {
    pub(super) fn new(unit: UnitId, nodes: Vec<NodeId>, loc: &'a Locations,nw: &'a Network) -> Tour<'a> {
        Tour{unit, nodes, loc, nw}
    }
}


impl<'a> fmt::Display for Tour<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "tour of {} with {} nodes", self.unit, self.nodes.len())
    }
}
