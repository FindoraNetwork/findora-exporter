use anyhow::{bail, Context, Result};
use prometheus::{proto::MetricFamily, Gauge, IntGauge, Registry};

use std::sync::Arc;

/// A wrapping collection for Metric structure.
pub(crate) struct Metrics {
    metrics: Vec<Arc<Metric>>,
}

impl Metrics {
    /// Returns a Metrics instance.
    ///
    /// This method registers Metric structures for managing easily.
    /// Returns error when registering Metric on failure.
    pub(crate) fn new(cfg: &crate::config::Crawler) -> Result<Self> {
        let mut metrics = vec![];
        for target in &cfg.targets {
            metrics
                .push(Arc::new(Metric::new(&target.registry).with_context(
                    || format!("new metric failed: {:?}", target),
                )?));
        }

        Ok(Metrics { metrics })
    }

    /// Returns a flattened vector of all metrics inside.
    pub(crate) fn gather(&self) -> Vec<MetricFamily> {
        let mut ret = vec![];
        for metric in &self.metrics {
            ret.push(metric.gather());
        }

        ret.into_iter().flatten().collect()
    }

    /// Returns an instance of Metric with Arc wrapping.
    /// Returns error when input index is out of range.
    pub(crate) fn get_metric(&self, index: usize) -> Result<Arc<Metric>> {
        if index >= self.metrics.len() {
            bail!("get_metric index out of range: {}", index)
        }
        Ok(self.metrics[index].clone())
    }
}

/// A wrapping structure for Prometheus library
pub(crate) struct Metric {
    registry: Registry,
    consensus_power: Gauge,
    network_functional: IntGauge,
    total_validators: IntGauge,
}

impl Default for Metric {
    fn default() -> Self {
        Metric {
            registry: Registry::new(),
            consensus_power: Gauge::new(
                "consensus_power",
                "percentage of the current consensus network voting power",
            )
            .unwrap(),
            network_functional: IntGauge::new(
                "network_functional",
                "subtraction of seconds of the latest block time with the current time",
            )
            .unwrap(),
            total_validators: IntGauge::new(
                "total_validators",
                "the total number of validators from the consensus network",
            )
            .unwrap(),
        }
    }
}

impl Metric {
    /// Returns a Metric instance.
    ///
    /// Registers a custom registry if not None in the config file,
    /// if None then registers a default registry instead.
    fn new(cfg: &Option<crate::config::Registry>) -> Result<Self> {
        let mut m = Metric::default();
        if let Some(c) = cfg {
            m.registry = Registry::new_custom(Some(c.prefix.clone()), Some(c.labels.clone()))
                .with_context(|| format!("new custom registry failed: {:?}", c))?;
        }

        m.registry
            .register(Box::new(m.consensus_power.clone()))
            .context("register consensus_power failed")?;
        m.registry
            .register(Box::new(m.network_functional.clone()))
            .context("register network_functional failed")?;
        m.registry
            .register(Box::new(m.total_validators.clone()))
            .context("register total_validators failed")?;

        Ok(m)
    }

    fn gather(&self) -> Vec<MetricFamily> {
        self.registry.gather()
    }

    /// set consensus power Gauge metric.
    pub(crate) fn set_consensus_power(&self, v: f64) {
        self.consensus_power.set(v)
    }

    /// set network functional IntGauge metric.
    pub(crate) fn set_network_functional(&self, v: i64) {
        self.network_functional.set(v)
    }

    /// set total validators IntGauge metric.
    pub(crate) fn set_total_validators(&self, v: i64) {
        self.total_validators.set(v)
    }
}
