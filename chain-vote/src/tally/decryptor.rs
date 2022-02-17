use crate::{Tally, TallyOptimizationTable};

use super::{TallyError, ValidatedTally};

pub struct TallyDecryptor(Option<TallyOptimizationTable>);

impl TallyDecryptor {
    pub fn with_abs_max_votes(absolute_max_votes: u64) -> Self {
        match absolute_max_votes {
            0 => Self(None),
            max => {
                let table = TallyOptimizationTable::generate_with_balance(
                    max.try_into().unwrap(),
                    1.try_into().unwrap(),
                );
                Self(Some(table))
            }
        }
    }

    pub fn decrypt(
        &self,
        validated_tally: &ValidatedTally,
        max_votes: u64,
    ) -> Result<Tally, TallyError> {
        match (&self.0, max_votes.try_into()) {
            (Some(table), Ok(max_votes)) => validated_tally.decrypt_tally(table, max_votes),
            _ => {
                let votes = vec![0; validated_tally.len()];
                Ok(Tally { votes })
            }
        }
    }
}


