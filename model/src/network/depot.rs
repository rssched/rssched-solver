use std::collections::HashMap;

use crate::base_types::{DepotId, Location, VehicleCount, VehicleTypeId};

pub struct Depot {
    depot_id: DepotId,
    location: Location,
    total_capacity: VehicleCount,
    allowed_types: HashMap<VehicleTypeId, Option<VehicleCount>>, // number of vehicles that can be
                                                                 // spawned. None means no limit.
}

// methods
impl Depot {
    pub fn depot_id(&self) -> DepotId {
        self.depot_id
    }

    pub fn location(&self) -> Location {
        self.location
    }

    pub fn total_capacity(&self) -> VehicleCount {
        self.total_capacity
    }

    /// takes the minimum of vehicle specific capacity (None means no limit) and depot capacity
    pub fn capacity_for(&self, vehicle_type_id: VehicleTypeId) -> VehicleCount {
        match self.allowed_types.get(&vehicle_type_id) {
            Some(Some(capacity)) => VehicleCount::min(*capacity, self.total_capacity),
            Some(None) => self.total_capacity, // no vehicle specific limit
            None => 0,                         // vehicle type not allowed
        }
    }
}

// static
impl Depot {
    pub fn new(
        depot_id: DepotId,
        location: Location,
        total_capacity: VehicleCount,
        allowed_types: HashMap<VehicleTypeId, Option<VehicleCount>>,
    ) -> Self {
        Self {
            depot_id,
            location,
            total_capacity,
            allowed_types,
        }
    }
}
