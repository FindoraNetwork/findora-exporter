use anyhow::{bail, Context, Result};
use prometheus::{proto::MetricFamily, Gauge, IntGauge, Registry};

use std::sync::Arc;

pub(crate) struct Metrics {
    metrics: Vec<Arc<Metric>>,
}

impl Metrics {
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

    pub(crate) fn gather(&self) -> Vec<MetricFamily> {
        let mut ret = vec![];
        for metric in &self.metrics {
            ret.push(metric.gather());
        }

        ret.into_iter().flatten().collect()
    }

    pub(crate) fn get_consensus_power(&self, index: usize) -> Result<Arc<Metric>> {
        if index >= self.metrics.len() {
            bail!("get_consensus_power index out of range: {}", index)
        }
        Ok(self.metrics[index].clone())
    }
}

pub(crate) struct Metric {
    registry: Registry,
    consensus_power: Gauge,
    network_functional: IntGauge,
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
        }
    }
}

impl Metric {
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
            .context("register network functional failed")?;

        Ok(m)
    }

    fn gather(&self) -> Vec<MetricFamily> {
        self.registry.gather()
    }

    pub(crate) fn set_consensus_power(&self, v: f64) {
        self.consensus_power.set(v)
    }

    pub(crate) fn set_network_functional(&self, v: i64) {
        self.network_functional.set(v)
    }
}
