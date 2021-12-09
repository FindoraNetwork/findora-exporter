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

Please download the suitable asset from

#### findora-exporter [prebuilt]

This project follows [Semantic Versioning]

### From Deb Package

Install it by below command

```bash
dpkg --install findora-exporter_x.x.x_amd64.deb
```

After that
* An executable binary will be put into `/usr/local/bin/findora-exporter`
* A systemd `findora-exporter.service` will be loaded
* expecting a config file at `/etc/prometheus/findora_exporter_config.json`

```bash
$ systemctl status findora-exporter.service
● findora-exporter.service - Findora Exporter for Prometheus
     Loaded: loaded (/lib/systemd/system/findora-exporter.service; enabled; vendor preset: enabled)
     Active: inactive (dead)
```

### From Tarball

Extracting it by below command

```bash
tar -xzf findora-exporter-x.x.x-x86_64-unknown-linux-musl.tar.gz
```

### From Source Code

Installing [Rust]

* Running test by
```bash
$ cargo test --all --all-features --no-fail-fast
test result: ok. x passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

* Building it by
```bash
cargo build --release
```

### Default Configuration Behavior

* listening `127.0.0.1:9090` address for Prometheus scraping
* crawling `http://127.0.0.1:26657` every 3 seconds
* displaying `trace` level information

### Specific A Configuration

for example
```json
{
    "log_level": "error",
    "server": {
	"listen_addr": "127.0.0.1:8080"
    },
    "crawler": {
        "targets": [
            {
                "host_addr": "https://prod-testnet.prod.findora.org:26657",
		"registry": {
                    "prefix": "findora_exporter",
		    "env": "prod-testnet"
		}
            },
            {
                "host_addr": "https://prod-mainnet.prod.findora.org:26657",
		"registry": {
		    "prefix": "findora_exporter",
		    "env": "prod-mainnet"
		}
            }
        ]
    }
}
```

[Semantic Versioning]: https://semver.org/#semantic-versioning-200
[prebuilt]: https://github.com/FindoraNetwork/findora-exporter/releases
[Rust]: https://www.rust-lang.org/learn/get-started

## Contributing
This project's goal is to provide the custom metrics by
* Small size of binary as possible
* Reasonable responding time for Prometheus
* Reliable value for monitoring and alerting

I'd very admire the [ureq] project's this [maxim].

so, welcomes any kind of suggestions or ideas please just create an issue or make a PR!

[ureq]: https://github.com/algesten/ureq
[maxim]: https://github.com/algesten/ureq#blocking-io-for-simplicity
