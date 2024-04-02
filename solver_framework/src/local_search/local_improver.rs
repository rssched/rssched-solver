use objective_framework::EvaluatedSolution;
use objective_framework::{Objective, ObjectiveValue};
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::Mutex;

use super::LocalSearchable;

/// Determines for a given solution the best neighbor that has an improving objective function.
/// Returns None if there is no better solution in the neighborhood.
pub trait LocalImprover<S: LocalSearchable> {
    fn improve(&self, solution: &EvaluatedSolution<S>) -> Option<EvaluatedSolution<S>>;
}

///////////////////////////////////////////////////////////
////////////////////// Minimizer //////////////////////////
///////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct Minimizer<S: LocalSearchable> {
    objective: Arc<Objective<S>>,
}

impl<S: LocalSearchable> Minimizer<S> {
    pub fn new(objective: Arc<Objective<S>>) -> Minimizer<S> {
        Minimizer { objective }
    }
}

impl<S: LocalSearchable> LocalImprover<S> for Minimizer<S> {
    fn improve(&self, solution: &EvaluatedSolution<S>) -> Option<EvaluatedSolution<S>> {
        let best_neighbor_opt = solution
            .solution()
            .neighborhood()
            .map(|neighbor| self.objective.evaluate(neighbor))
            .min_by(|s1, s2| {
                s1.objective_value()
                    .partial_cmp(s2.objective_value())
                    .unwrap()
            });
        match best_neighbor_opt {
            Some(best_neighbor) => {
                if best_neighbor.objective_value() < solution.objective_value() {
                    Some(best_neighbor)
                } else {
                    None // no improvement found
                }
            }
            None => {
                println!("\x1b[31mWARNING: NO SWAP POSSIBLE.\x1b[0m");
                None
            }
        }
    }
}

///////////////////////////////////////////////////////////
///////////////// TakeFirstRecursion //////////////////////
///////////////////////////////////////////////////////////

/// Find the first improving solution in the neighborhood of the given solution.
/// As there is no parallelization this improver is fully deterministic.
#[derive(Clone)]
pub struct TakeFirstRecursion<S: LocalSearchable> {
    recursion_depth: u8,
    recursion_width: Option<usize>, // number of schedule that are considered for recursion (the one with best value are taken)
    objective: Arc<Objective<S>>,
}

impl<S: LocalSearchable> LocalImprover<S> for TakeFirstRecursion<S> {
    fn improve(&self, solution: &EvaluatedSolution<S>) -> Option<EvaluatedSolution<S>> {
        let old_objective_value = solution.objective_value();
        self.improve_recursion(
            vec![solution.clone()],
            old_objective_value,
            self.recursion_depth,
        )
    }
}

impl<S: LocalSearchable> TakeFirstRecursion<S> {
    pub fn new(
        recursion_depth: u8,
        recursion_width: Option<usize>,
        objective: Arc<Objective<S>>,
    ) -> TakeFirstRecursion<S> {
        TakeFirstRecursion {
            recursion_depth,
            recursion_width,
            objective,
        }
    }

    /// Returns the first improving solution in the neighborhood of the given solutions.
    /// If no improvement is found, None is returned.
    fn improve_recursion(
        &self,
        solutions: Vec<EvaluatedSolution<S>>,
        objective_to_beat: &ObjectiveValue,
        remaining_recursion: u8,
    ) -> Option<EvaluatedSolution<S>> {
        let neighboorhood_union = solutions
            .iter()
            .flat_map(|sol| sol.solution().neighborhood());

        let mut counter = 0;
        let mut solutions_for_recursion: Vec<EvaluatedSolution<S>> = Vec::new();

        let result = neighboorhood_union
            .map(|neighbor| {
                counter += 1;
                self.objective.evaluate(neighbor)
            })
            .find(|neighbor| {
                if remaining_recursion > 0 {
                    solutions_for_recursion.push(neighbor.clone());
                    if let Some(width) = self.recursion_width {
                        solutions_for_recursion.sort();
                        solutions_for_recursion.dedup();
                        // schedules_for_recursion.dedup_by(|s1,s2| s1.cmp_objective_values(s2).is_eq()); //remove dublicates
                        let width = width.min(solutions_for_recursion.len());
                        solutions_for_recursion.truncate(width);
                    }
                }
                neighbor.objective_value() < objective_to_beat
            });

        if result.is_none() {
            println!("No improvement found after {} swaps.", counter);

            if remaining_recursion > 0 {
                println!(
                    "Going into recursion. Remaining depth: {}. Schedule-count: {}",
                    remaining_recursion,
                    solutions_for_recursion.len()
                );

                self.improve_recursion(
                    solutions_for_recursion,
                    objective_to_beat,
                    remaining_recursion - 1,
                )
            } else {
                println!("No recursion-depth left.");
                None
            }
        } else {
            println!("Improvement found after {} swaps.", counter);
            result
        }
    }
}

///////////////////////////////////////////////////////////
/////////////// TakeAnyParallelRecursion //////////////////
///////////////////////////////////////////////////////////

/// This improver uses parallel computation at two steps. In the recursion when multiple solutions
/// are given, each solution get its own thread. Within each thread the neighborhood iterator is tranformed
/// to a ParallelIterator (messes up the ordering) and search for ANY improving solution in
/// parallel.
/// As soon as an improving soltion is found a terminus-signal is broadcast to all other solutions.
/// If no improving solution is found the width-many solutions of each thread are take to recursion
/// (dublicates are removed)
/// Due to the parallel computation and find_any() this improver is the fastest but not
/// deterministic.
#[derive(Clone)]
pub struct TakeAnyParallelRecursion<S: LocalSearchable> {
    recursion_depth: u8,
    recursion_width: Option<usize>, // number of schedule that are considered per schedule for the next recursion (the one with best objectivevalue are taken for each schedule, dublicates are removed)
    objective: Arc<Objective<S>>,
}

impl<S: LocalSearchable> LocalImprover<S> for TakeAnyParallelRecursion<S> {
    fn improve(&self, solution: &EvaluatedSolution<S>) -> Option<EvaluatedSolution<S>> {
        let old_objective = solution.objective_value();
        self.improve_recursion(vec![solution.clone()], old_objective, self.recursion_depth)
    }
}

impl<S: LocalSearchable> TakeAnyParallelRecursion<S> {
    pub fn new(
        recursion_depth: u8,
        recursion_width: Option<usize>,
        objective: Arc<Objective<S>>,
    ) -> TakeAnyParallelRecursion<S> {
        TakeAnyParallelRecursion {
            recursion_depth,
            recursion_width,
            objective,
        }
    }

    fn improve_recursion(
        &self,
        solutions: Vec<EvaluatedSolution<S>>,
        objective_to_beat: &ObjectiveValue,
        remaining_recursion: u8,
    ) -> Option<EvaluatedSolution<S>> {
        let mut solution_collection: Vec<Vec<EvaluatedSolution<S>>> = Vec::new();
        let mut result: Option<EvaluatedSolution<S>> = None;
        rayon::scope(|s| {
            let mut found_senders = Vec::new();
            let (success_sender, success_receiver) = channel();
            let (failure_sender, failure_receiver) = channel();

            for sol in solutions.iter() {
                let (found_sender, found_receiver) = channel();
                found_senders.push(found_sender);

                let succ_sender = success_sender.clone();
                let fail_sender = failure_sender.clone();
                s.spawn(move |_| {
                    let found_receiver_mutex = Arc::new(Mutex::new(found_receiver));

                    let mut new_solutions: Vec<EvaluatedSolution<S>> = Vec::new();
                    let new_solutions_mutex: Arc<Mutex<&mut Vec<EvaluatedSolution<S>>>> =
                        Arc::new(Mutex::new(&mut new_solutions));

                    let result = sol
                        .solution()
                        .neighborhood()
                        .par_bridge()
                        .map(|neighbor| self.objective.evaluate(neighbor))
                        .find_any(|evaluated_neighbor| {
                            if remaining_recursion > 0 {
                                let mut schedules_mutex = new_solutions_mutex.lock().unwrap();

                                schedules_mutex.push(evaluated_neighbor.clone());

                                // if there is a recursion_width truncate schedules to the best width many
                                if let Some(width) = self.recursion_width {
                                    schedules_mutex.sort();
                                    // schedules_mutex.dedup(); //remove dublicates
                                    schedules_mutex.dedup_by(|s1, s2| {
                                        s1.objective_value().cmp(s2.objective_value()).is_eq()
                                    }); //remove dublicates according to objective_value
                                    let width = width.min(schedules_mutex.len());
                                    schedules_mutex.truncate(width);
                                }
                            }

                            let found_receiver_mutex = found_receiver_mutex.lock().unwrap();
                            let found = found_receiver_mutex.try_recv();
                            evaluated_neighbor
                                .objective_value()
                                .cmp(objective_to_beat)
                                .is_lt()
                                || found.is_ok()
                        });

                    match result {
                        Some(sol) => {
                            if sol.objective_value() < objective_to_beat {
                                succ_sender.send(sol).unwrap();
                            }
                            // if there is a Some result but the objective is not better, that means
                            // another thread was successful first. So there is nothing
                            // left to do for this thread.
                        }
                        None => {
                            fail_sender.send(new_solutions).unwrap();
                        }
                    }
                });
            }

            drop(success_sender);
            drop(failure_sender);

            while let Ok(new_sol_pair) = success_receiver.recv() {
                for s in found_senders.iter() {
                    s.send(true).ok();
                }
                if result.is_none()
                    || new_sol_pair.objective_value() < result.as_ref().unwrap().objective_value()
                {
                    result = Some(new_sol_pair);
                }
            }
            if result.is_none() {
                for v in failure_receiver.into_iter() {
                    solution_collection.push(v);
                }
            }
        });

        if result.is_none() {
            // println!("No improvement found.");

            if remaining_recursion > 0 {
                let mut schedules_for_recursion: Vec<EvaluatedSolution<S>> =
                    solution_collection.into_iter().flatten().collect();

                schedules_for_recursion.sort();
                // schedules_for_recursion.dedup(); //remove dublicates
                schedules_for_recursion.dedup_by(|s1, s2| s1.cmp(&s2).is_eq()); //remove dublicates according to objective_value

                self.improve_recursion(
                    schedules_for_recursion,
                    objective_to_beat,
                    remaining_recursion - 1,
                )
            } else {
                // println!("No recursion-depth left.");
                None
            }
        } else {
            // println!("Improvement found.");
            result
        }
    }
}