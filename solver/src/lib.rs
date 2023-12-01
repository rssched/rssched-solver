mod first_phase_objective;
mod solver;

use sbb_solution::json_serialisation::write_solution_to_json;
use solver::greedy::Greedy;
use solver::Solver;
// use solver::local_search::LocalSearch;

use sbb_model::json_serialisation::load_rolling_stock_problem_instance_from_json;

use std::sync::Arc;
use std::time as stdtime;

pub fn run(path: &str) {
    println!("\n\n********** RUN: {} **********\n", path);

    let (locations, vehicle_types, network, config) =
        load_rolling_stock_problem_instance_from_json(path);
    let start_time = stdtime::Instant::now();

    let objective = Arc::new(first_phase_objective::build());

    // initialize local search
    // let mut local_search_solver =
    // LocalSearch::initialize(config.clone(), vehicle_types.clone(), network.clone());

    // use greedy algorithm
    let greedy = Greedy::initialize(
        vehicle_types.clone(),
        network.clone(),
        config.clone(),
        objective.clone(),
    );

    // solve
    let final_solution = greedy.solve();

    let end_time = stdtime::Instant::now();
    let runtime_duration = end_time.duration_since(start_time);

    println!("\n\nFinal schedule (long version):");
    final_solution.solution().print_tours_long();

    println!("\n\nFinal schedule:");
    final_solution.solution().print_tours();

    println!("\n\nTrain formations:");
    final_solution.solution().print_train_formations();
    println!();
    println!("\nObjective value::");
    objective.print_objective_value(final_solution.objective_value());

    println!("Running time: {:0.2}sec", runtime_duration.as_secs_f32());

    let output_name = format!("output_{}", path); // TODO: make this work with sub-directories
    write_solution_to_json(&final_solution, &objective, &output_name).expect("Error writing json");
}
