use crate::{types::Inspection, Inspector};
use ethers::types::Trace;
use itertools::Itertools;

/// Classifies traces according to the provided inspectors
pub struct BatchInspector {
    inspectors: Vec<Box<dyn Inspector>>,
}

impl BatchInspector {
    /// Constructor
    pub fn new(inspectors: Vec<Box<dyn Inspector>>) -> Self {
        Self { inspectors }
    }

    /// Given a trace iterator, it groups all traces for the same tx hash
    /// and then inspects them and all of their subtraces
    pub fn inspect_many<'a>(&'a self, traces: impl IntoIterator<Item = Trace>) -> Vec<Inspection> {
        // group traces in a block by tx hash
        let traces = traces.into_iter().group_by(|t| t.transaction_hash);

        let inspections = traces
            .into_iter()
            // Convert the traces to inspections
            .map(|(_, traces)| Inspection::from(traces))
            // Make an unclassified inspection per tx_hash containing a tree of traces
            .map(|mut i| {
                self.inspect(&mut i);
                i
            })
            .collect();

        // TODO: Convert these inspections to known/unknown Evaluations
        // containing profit-related data.
        inspections
    }

    // pub fn reduce(&self, inspection: Inspection) {}

    /// Decodes the inspection's actions
    pub fn inspect(&self, inspection: &mut Inspection) {
        for inspector in self.inspectors.iter() {
            inspector.inspect(inspection);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        addresses::ADDRESSBOOK,
        inspectors::{Aave, Uniswap},
        test_helpers::*,
    };
    use ethers::types::U256;

    #[test]
    #[ignore]
    // call that starts from a bot but has a uniswap sub-trace
    // https://etherscan.io/tx/0x93690c02fc4d58734225d898ea4091df104040450c0f204b6bf6f6850ac4602f
    // 99k USDC -> 281 ETH -> 5.7 YFI trade
    // Liquidator Repay -> 5.7 YFI
    // Liquidation -> 292 ETH
    // Profit: 11 ETH
    fn subtrace_parse() {
        let mut inspection =
            get_trace("0x93690c02fc4d58734225d898ea4091df104040450c0f204b6bf6f6850ac4602f");

        let inspector = BatchInspector::new(vec![Box::new(Uniswap::new()), Box::new(Aave::new())]);
        inspector.inspect(&mut inspection);

        let known = inspection.known();

        let liquidation = known
            .iter()
            .find_map(|action| action.as_ref().profitable_liquidation())
            .unwrap();
        assert_eq!(
            liquidation.profit,
            U256::from_dec_str("11050220339336343871").unwrap()
        );

        assert_eq!(ADDRESSBOOK.get(&liquidation.token).unwrap(), "ETH");
        assert_eq!(
            ADDRESSBOOK.get(&liquidation.as_ref().sent_token).unwrap(),
            "YFI"
        );
    }
}