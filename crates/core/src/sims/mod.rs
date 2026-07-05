//! Built-in sims. Each module implements the [`crate::sim::Sim`] contract.

pub mod big_bang;
pub mod black_hole;
pub mod carburetor;
pub mod supernova;
pub mod tribbles;
pub mod wing;

use crate::sim::Sim;

/// Every built-in sim, in display order.
#[must_use]
pub fn all_sims() -> Vec<Box<dyn Sim>> {
    vec![
        Box::new(tribbles::Tribbles::new()),
        Box::new(wing::Wing::new()),
        Box::new(carburetor::Carburetor::new()),
        Box::new(black_hole::BlackHole::new()),
        Box::new(supernova::Supernova::new()),
        Box::new(big_bang::BigBang::new()),
    ]
}

/// Find a sim by its id.
#[must_use]
pub fn sim_by_id(id: &str) -> Option<Box<dyn Sim>> {
    all_sims().into_iter().find(|sim| sim.meta().id == id)
}

#[cfg(test)]
mod tests {
    use super::{all_sims, sim_by_id};

    #[test]
    fn ids_are_unique_and_findable() {
        let sims = all_sims();
        assert!(sims.len() >= 3);
        for sim in &sims {
            let id = sim.meta().id;
            assert!(sim_by_id(id).is_some(), "{id} not findable");
        }
        assert!(sim_by_id("no-such-sim").is_none());
    }
}
