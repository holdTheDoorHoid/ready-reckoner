//! Facts about the household that the rules count: people by age band, special needs, pets,
//! vehicles and commuters.

use rr_types::{AgeBand, BackupPower, Commute, Fuel, Person, Pets, PlanInput};

/// Counts the rules need, read from a [`PlanInput`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct Household<'a> {
    pub input: &'a PlanInput,
}

/// A baby whose household noted formula in its dietary needs.
pub(crate) fn is_formula_fed(p: &Person) -> bool {
    p.age_band == AgeBand::Infant
        && p.medical
            .dietary
            .iter()
            .any(|d| d.to_lowercase().contains("formula"))
}

/// Aged 4 and over (can carry a light, a bag, a mask, a contact card).
pub(crate) fn is_4_plus(p: &Person) -> bool {
    !matches!(p.age_band, AgeBand::Infant | AgeBand::Toddler)
}

/// Aged 13 and over (usually has a phone and can use a radio).
pub(crate) fn is_13_plus(p: &Person) -> bool {
    matches!(p.age_band, AgeBand::Teen | AgeBand::Adult | AgeBand::Senior)
}

impl<'a> Household<'a> {
    pub(crate) fn new(input: &'a PlanInput) -> Self {
        Self { input }
    }

    pub(crate) fn people(&self) -> &'a [Person] {
        &self.input.people
    }

    pub(crate) fn pets(&self) -> &'a Pets {
        &self.input.pets
    }

    pub(crate) fn n(&self) -> usize {
        self.input.people.len()
    }

    /// Dogs, cats and small pets (not livestock).
    pub(crate) fn household_pets(&self) -> u32 {
        let p = self.pets();
        u32::from(p.dogs) + u32::from(p.cats) + u32::from(p.small)
    }

    pub(crate) fn vehicles(&self) -> usize {
        self.input.mobility.vehicles.len()
    }

    pub(crate) fn evs(&self) -> usize {
        self.input
            .mobility
            .vehicles
            .iter()
            .filter(|v| v.fuel == Fuel::Ev)
            .count()
    }

    pub(crate) fn has_generator(&self) -> bool {
        self.input.housing.backup_power == BackupPower::Generator
    }

    /// Everyone with a regular trip away from home, with their 1-based position in `people`.
    pub(crate) fn commuters(&self) -> Vec<(usize, &'a Commute)> {
        self.input
            .people
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                p.commute
                    .as_ref()
                    .filter(|c| c.distance_km.is_finite() && c.distance_km > 0.0)
                    .map(|c| (i + 1, c))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_counts() {
        let input = fixtures::get("philadelphia-renters-4").unwrap();
        let h = Household::new(&input);
        assert_eq!(h.n(), 4);
        assert_eq!(h.people().iter().filter(|p| is_4_plus(p)).count(), 4);
        assert_eq!(h.people().iter().filter(|p| is_13_plus(p)).count(), 3);
        assert_eq!(h.household_pets(), 1);
        let c = h.commuters();
        assert_eq!(c.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn sugar_land_formula_baby_and_ev() {
        let input = fixtures::get("sugar-land-ev-household-3").unwrap();
        let h = Household::new(&input);
        assert_eq!(h.people().iter().filter(|p| is_formula_fed(p)).count(), 1);
        assert_eq!(h.evs(), 1);
        assert_eq!(h.vehicles(), 2);
    }

    #[test]
    fn coos_bay_commuter_at_zero_km_is_not_a_commuter() {
        let input = fixtures::get("coos-bay-well-owner-2").unwrap();
        let h = Household::new(&input);
        assert_eq!(h.commuters().len(), 1);
        assert!(h.has_generator());
    }
}
