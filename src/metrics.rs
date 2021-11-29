use anyhow::{bail, Context, Result};
use prometheus::{proto::MetricFamily, Gauge, Registry};

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
}

impl Default for Metric {
    fn default() -> Self {
        Metric {
            registry: Registry::new(),
            consensus_power: Gauge::new(
                "consensus_power",
                "current consensus network voting power",
            )
            .unwrap(),
        }
    }
}

impl Metric {
    fn new(cfg: &Option<crate::config::Registry>) -> Result<Self> {
        let mut m = Metric::default();

        // TODO: beauty this match conditions when metric becomes more
        match cfg {
            Some(v) => {
                // using custom Registry
                let r = Registry::new_custom(Some(v.prefix.clone()), Some(v.labels.clone()))
                    .with_context(|| format!("new custom registry failed: {:?}", v))?;
                let consensus_power =
                    Gauge::new("consensus_power", "current consensus network voting power")
                        .context("consensus_power gauge failed")?;
                r.register(Box::new(consensus_power.clone()))
                    .context("custom registry registry consensus_power failed")?;
                m.registry = r;
                m.consensus_power = consensus_power;
            }
            None => {
                // using default Registry
                let r = Registry::new();
                let consensus_power =
                    Gauge::new("consensus_power", "current consensus network voting power")
                        .context("consensus_power gauge failed")?;
                r.register(Box::new(consensus_power.clone()))
                    .context("default registry registry consensus_power failed")?;
                m.registry = r;
                m.consensus_power = consensus_power;
            }
        }

        Ok(m)
    }

    fn gather(&self) -> Vec<MetricFamily> {
        self.registry.gather()
    }

    pub(crate) fn set_consensus_power(&self, v: f64) {
        self.consensus_power.set(v)
    }
}
