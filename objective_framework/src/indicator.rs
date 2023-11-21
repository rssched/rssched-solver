use crate::base_value::BaseValue;

/// An atomic aspect of the solution.
/// An indicator could be the aspect "number of dummy_tours" or "total deadhead distance", ...
pub trait Indicator<S> {
    fn evaluate(&self, solution: &S) -> BaseValue;
    fn name(&self) -> String;
}
