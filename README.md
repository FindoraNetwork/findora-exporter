# findora-exporter
[![ci status](https://github.com/FindoraNetwork/findora-exporter/actions/workflows/main.yml/badge.svg?branch=main)](https://github.com/FindoraNetwork/findora-exporter/actions)

a Prometheus exporter for Findora Network

## Features
Findora is using [tendermint] for its own consensus network. tendermint already exposed a lot of wonderful [metrics] but there have some customize requests that needs to be monitored.

[tendermint]: https://tendermint.com/
[metrics]: https://docs.tendermint.com/master/nodes/metrics.html

This exporter has below custom metrics right now!

| name | type | help |
| - | :-: | :-: |
| consensus_power | Generic Gauge | percentage of the current consensus network voting power |
| network_functional | Int Gauge | subtraction of seconds of the latest block time with the current time |
| total_validators | Int Gauge | the total number of validators from the consensus network |

## Installation

## Contributing
This project's goal is to provide the custom metrics by
* Small size of binary as possible
* Reasonable responding time for Prometheus
* Reliable value for monitoring and alerting

I'd very admire the [ureq] project's this [maxim].

so, welcomes any kind of suggestions or ideas please just create an issue or make a PR!

[ureq]: https://github.com/algesten/ureq
[maxim]: https://github.com/algesten/ureq#blocking-io-for-simplicity
