use anyhow::{bail, Context, Result};
use prometheus::{
    core::{Atomic, GenericGauge},
    proto::MetricFamily,
    Registry,
};

use crate::{config::TaskName, utils::calculate_hash};

use std::{collections::HashMap, sync::Arc};

/// A wrapping collection for Metric structure.
pub(crate) struct Metrics<T: Atomic> {
    metrics: HashMap<u64, Arc<Metric<T>>>,
}

impl<T> Metrics<T>
where
    T: Atomic + 'static,
{
    /// Returns a Metrics instance.
    ///
    /// This method registers Metric structures for managing easily.
    /// Returns error when registering Metric on failure.
    pub(crate) fn new(cfg: &crate::config::Crawler) -> Result<Self> {
        let mut metrics = HashMap::with_capacity(cfg.targets.len());
        for target in &cfg.targets {
            metrics.insert(
                calculate_hash(target),
                Arc::new(
                    Metric::new(target)
                        .with_context(|| format!("new metric failed: {:?}", target))?,
                ),
            );
        }

        Ok(Metrics { metrics })
    }

    /// Returns a flattened vector of all metrics inside.
    pub(crate) fn gather(&self) -> Vec<MetricFamily> {
        self.metrics
            .values()
            .flat_map(|metric| metric.gather())
            .collect()
    }

    /// Returns an instance of Metric with Arc wrapping.
    pub(crate) fn get_metric(&self, hash: u64) -> Result<Arc<Metric<T>>> {
        match self.metrics.get(&hash) {
            Some(metric) => Ok(metric.clone()),
            None => bail!("get_metric not found: {}", hash),
        }
    }
}

/// A wrapping structure for Prometheus library
pub(crate) struct Metric<T: Atomic> {
    registry: Registry,
    metric: GenericGauge<T>,
}

impl<T> Default for Metric<T>
where
    T: Atomic,
{
    fn default() -> Self {
        Metric {
            registry: Registry::new(),
            metric: GenericGauge::new(
                "network_functional",
                "subtraction of seconds of the latest block time with the current time",
            )
            .unwrap(),
        }
    }
}

impl<T> Metric<T>
where
    T: Atomic + 'static,
{
    /// Returns a Metric instance.
    ///
    /// Registers a custom registry if not None in the config file,
    /// if None then registers a default registry instead.
    fn new(cfg: &crate::config::Target) -> Result<Self> {
        let registry = match &cfg.registry {
            Some(r) => Registry::new_custom(Some(r.prefix.clone()), Some(r.labels.clone()))
                .with_context(|| format!("new custom registry failed: {:?}", r))?,
            None => Registry::new(),
        };

        let metric = match cfg.task_name {
            TaskName::ConsensusPower => GenericGauge::new(
                "consensus_power",
                "percentage of the current consensus network voting power",
            )
            .context("new consensus_power failed")?,
            TaskName::NetworkFunctional => GenericGauge::new(
                "network_functional",
                "subtraction of seconds of the latest block time with the current time",
            )
            .context("new network_functional failed")?,
            TaskName::TotalCountOfValidators => GenericGauge::new(
                "total_count_of_validators",
                "the total number of validators from the consensus network",
            )
            .context("new total_count_of_validators failed")?,
            TaskName::TotalBalanceOfRelayers => GenericGauge::new(
                "total_balance_of_relayers",
                "the total balance of relayers from the specific bridge",
            )
            .context("new total_balance_of_relayers failed")?,
            TaskName::BridgedBalance => GenericGauge::new(
                "bridged_balance",
                "the token balance of reserving safe on source chain",
            )
            .context("new bridged_balance failed")?,
            TaskName::BridgedSupply => GenericGauge::new(
                "bridged_supply",
                "the token supply total minted on the destination chain",
            )
            .context("new bridged_supply failed")?,
            TaskName::NativeBalance => GenericGauge::new(
                "native_balance",
                "the native balance of reserving safe on source chain",
            )
            .context("new native_balance failed")?,
            TaskName::GetPrice => GenericGauge::new(
                "get_price",
                "the close price of the related currency pair from gate.io",
            )
            .context("new get_price failed")?,
        };

        registry
            .register(Box::new(metric.clone()))
            .context("register metric failed")?;

        Ok(Metric { registry, metric })
    }

    fn gather(&self) -> Vec<MetricFamily> {
        self.registry.gather()
    }

    /// set a value for metric
    pub(crate) fn set(&self, v: <T as Atomic>::T) {
        self.metric.set(v)
    }
}
