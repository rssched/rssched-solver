use derive_more::Display;
use derive_more::From;

pub mod distance;
pub mod location;

pub use distance::Distance;
pub use location::Location;

pub type Idx = u16;

#[derive(Display, From, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocationIdx(pub Idx);

#[derive(Display, From, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleTypeIdx(pub Idx);

#[derive(Display, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VehicleIdx {
    #[display(fmt = "veh_{}", _0)]
    Vehicle(Idx),
    #[display(fmt = "dummy_{}", _0)]
    Dummy(Idx),
}

impl VehicleIdx {
    pub fn vehicle_from(idx: Idx) -> VehicleIdx {
        VehicleIdx::Vehicle(idx)
    }
    pub fn dummy_from(idx: Idx) -> VehicleIdx {
        VehicleIdx::Dummy(idx)
    }

    pub fn idx(&self) -> Idx {
        match self {
            VehicleIdx::Vehicle(idx) => *idx,
            VehicleIdx::Dummy(idx) => *idx,
        }
    }
}

#[derive(Display, From, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DepotIdx(pub Idx);

#[derive(Display, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeIdx {
    #[display(fmt = "sdep_{}", _0)]
    StartDepot(Idx),
    #[display(fmt = "serv_{}", _0)]
    Service(Idx),
    #[display(fmt = "main_{}", _0)]
    Maintenance(Idx),
    #[display(fmt = "edep_{}", _0)]
    EndDepot(Idx),
}

impl NodeIdx {
    pub fn start_depot_from(idx: Idx) -> NodeIdx {
        NodeIdx::StartDepot(idx)
    }
    pub fn end_depot_from(idx: Idx) -> NodeIdx {
        NodeIdx::EndDepot(idx)
    }
    pub fn service_from(idx: Idx) -> NodeIdx {
        NodeIdx::Service(idx)
    }
    pub fn maintenance_from(idx: Idx) -> NodeIdx {
        NodeIdx::Maintenance(idx)
    }
    pub fn smallest() -> NodeIdx {
        NodeIdx::StartDepot(0)
    }
    pub fn idx(&self) -> Idx {
        match self {
            NodeIdx::StartDepot(idx) => *idx,
            NodeIdx::Service(idx) => *idx,
            NodeIdx::Maintenance(idx) => *idx,
            NodeIdx::EndDepot(idx) => *idx,
        }
    }
}

pub type VehicleCount = u32;
pub type PassengerCount = u32;
pub type Meter = u64;
pub type Cost = u64;
pub const MAINT_COUNTER_FOR_INF_DIST: Meter = 1_000;
pub type MaintenanceCounter = i64;
