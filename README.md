# findora-exporter
[![ci status](https://github.com/FindoraNetwork/findora-exporter/actions/workflows/main.yml/badge.svg?branch=main)](https://github.com/FindoraNetwork/findora-exporter/actions)

a Prometheus exporter for Findora Network

## Features
Findora is using [tendermint](https://tendermint.com/) for its own consensus network. tendermint already exposed a lot of wonderful [metrics](https://docs.tendermint.com/master/nodes/metrics.html) but there have some customize requests that needs to be monitored.

This exporter has below custom metrics right now!

| name | type | help |
| - | :-: | :-: |
| consensus_power | Generic Gauge | percentage of the current consensus network voting power |
| network_functional | Int Gauge | subtraction of seconds of the latest block time with the current time |
| total_validators | Int Gauge | the total number of validators from the consensus network |

## Installation

## Contributing
