mod vehicles;

mod schedule;

mod solver;

use solver::Solver;
// use solver::greedy_1::Greedy1;
// use solver::greedy_2::Greedy2;
use solver::greedy_3::Greedy3;
use solver::local_search::LocalSearch;

use sbb_model::config::Config;
use sbb_model::locations::Locations;
use sbb_model::network::Network;
use sbb_model::vehicles::Vehicles;

use schedule::Schedule;

use std::sync::Arc;
use std::time as stdtime;

pub fn run(path: &str) {
    println!("\n\n********** RUN: {} **********\n", path);

    // load instance: All the objects are static and are multiple times referenced;
    // network also references Locations

    // TODO load config, loc, vehicles, and network from json

    let start_time = stdtime::Instant::now();

    // initialize local search
    let mut local_search_solver =
        LocalSearch::initialize(config.clone(), vehicles.clone(), nw.clone());

    // set initial_schedule:

    // use greedy algorithm
    let greedy_3 = Greedy3::initialize(config.clone(), vehicles.clone(), nw.clone());
    local_search_solver.set_initial_schedule(greedy_3.solve());

    // load SBB-schedule:
    // local_search_solver.set_initial_schedule(Schedule::load_from_csv(&format!("{}{}", path, "leistungsketten.csv"), config.clone(), Vehicles.clone(), nw.clone()));

    // execute local search:
    let final_schedule = local_search_solver.solve();

    let end_time = stdtime::Instant::now();
    let runtime_duration = end_time.duration_since(start_time);

    println!("\n\nFinal schedule (long version):");
    final_schedule.print_long();

    println!("\n\nFinal schedule:");
    final_schedule.print();
    println!();
    let optimal = nw.minimal_overhead();
    println!("min_overhead: {}", optimal);
    println!("ETH_Solution:");
    final_schedule.objective_value().print(None);

    println!("Running time: {:0.2}sec", runtime_duration.as_secs_f32());

    final_schedule
        .write_to_csv(&format!("{}{}", path, "ETH_leistungsketten.csv"))
        .unwrap();

    println!();
    let loaded_schedule = Schedule::load_from_csv(
        &format!("{}{}", path, "leistungsketten.csv"),
        config.clone(),
        vehicles.clone(),
        nw.clone(),
    );
    println!("SBB_Solution:");
    loaded_schedule.objective_value().print(None);
}
