# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.18](https://github.com/edera-dev/krata/compare/v0.0.17...v0.0.18) - 2024-08-22

### Added
- *(zone)* kernel command line control on launch ([#351](https://github.com/edera-dev/krata/pull/351))
- *(xen-preflight)* test for hypervisor presence explicitly and error if missing ([#347](https://github.com/edera-dev/krata/pull/347))

### Fixed
- *(network)* allocate host ip from allocation pool ([#353](https://github.com/edera-dev/krata/pull/353))
- *(daemon)* turn off trace logging ([#352](https://github.com/edera-dev/krata/pull/352))

### Other
- Add support for reading hypervisor console ([#344](https://github.com/edera-dev/krata/pull/344))
- *(ctl)* move logic for branching ctl run steps into ControlCommands ([#342](https://github.com/edera-dev/krata/pull/342))
- update Cargo.toml dependencies

## [0.0.17](https://github.com/edera-dev/krata/compare/v0.0.16...v0.0.17) - 2024-08-15

### Added
- *(krata)* first pass on cpu hotplug support ([#340](https://github.com/edera-dev/krata/pull/340))
- *(exec)* implement tty support (fixes [#335](https://github.com/edera-dev/krata/pull/335)) ([#336](https://github.com/edera-dev/krata/pull/336))
- *(krata)* dynamic resource allocation (closes [#298](https://github.com/edera-dev/krata/pull/298)) ([#333](https://github.com/edera-dev/krata/pull/333))

### Other
- update Cargo.toml dependencies

## [0.0.16](https://github.com/edera-dev/krata/compare/v0.0.15...v0.0.16) - 2024-08-14

### Added
- *(krata)* prepare for workload rework ([#276](https://github.com/edera-dev/krata/pull/276))

### Fixed
- *(idm)* reimplement packet processing algorithm ([#330](https://github.com/edera-dev/krata/pull/330))
- *(power-trap-eacces)* gracefully handle hypercall errors in power management ([#325](https://github.com/edera-dev/krata/pull/325))

### Other
- *(o11y)* add more debug logs to daemon & runtime ([#318](https://github.com/edera-dev/krata/pull/318))

## [0.0.15](https://github.com/edera-dev/krata/compare/v0.0.14...v0.0.15) - 2024-08-06

### Fixed
- *(zone)* waitpid should be limited when no child processes exist (fixes [#304](https://github.com/edera-dev/krata/pull/304)) ([#305](https://github.com/edera-dev/krata/pull/305))

## [0.0.14](https://github.com/edera-dev/krata/compare/v0.0.13...v0.0.14) - 2024-08-06

### Added
- *(oci)* use local index as resolution cache when appropriate, fixes [#289](https://github.com/edera-dev/krata/pull/289) ([#294](https://github.com/edera-dev/krata/pull/294))

### Fixed
- *(idm)* process all idm messages in the same frame and use childwait exit notification for exec (fixes [#290](https://github.com/edera-dev/krata/pull/290)) ([#302](https://github.com/edera-dev/krata/pull/302))

### Other
- init: mount /proc with hidepid=1 ([#277](https://github.com/edera-dev/krata/pull/277))
- update Cargo.toml dependencies

## [0.0.13](https://github.com/edera-dev/krata/compare/v0.0.12...v0.0.13) - 2024-07-19

### Added
- *(kratactl)* rework cli to use subcommands ([#268](https://github.com/edera-dev/krata/pull/268))
- *(krata)* rename guest to zone ([#266](https://github.com/edera-dev/krata/pull/266))

### Other
- *(deps)* upgrade dependencies, fix hyper io traits issue ([#252](https://github.com/edera-dev/krata/pull/252))
- update Cargo.lock dependencies
- update Cargo.toml dependencies

## [0.0.12](https://github.com/edera-dev/krata/compare/v0.0.11...v0.0.12) - 2024-07-12

### Added
- *(oci)* add configuration value for oci seed file ([#220](https://github.com/edera-dev/krata/pull/220))
- *(power-management-defaults)* set an initial power management policy ([#219](https://github.com/edera-dev/krata/pull/219))

### Fixed
- *(daemon)* decrease rate of runtime reconcile ([#224](https://github.com/edera-dev/krata/pull/224))
- *(power)* ensure that xeon cpus with cpu gaps are not detected as p/e compatible ([#218](https://github.com/edera-dev/krata/pull/218))
- *(runtime)* use iommu only if devices are needed ([#243](https://github.com/edera-dev/krata/pull/243))

### Other
- Power management core functionality ([#217](https://github.com/edera-dev/krata/pull/217))
- *(powermgmt)* disable for now as a hackfix ([#242](https://github.com/edera-dev/krata/pull/242))
- Initial fluentd support ([#205](https://github.com/edera-dev/krata/pull/205))
- update Cargo.toml dependencies
- Use native loopdev implementation instead of loopdev-3 ([#209](https://github.com/edera-dev/krata/pull/209))

## [0.0.11](https://github.com/edera-dev/krata/compare/v0.0.10...v0.0.11) - 2024-06-23

### Added
- pci passthrough ([#114](https://github.com/edera-dev/krata/pull/114))
- *(runtime)* concurrent ip allocation ([#151](https://github.com/edera-dev/krata/pull/151))
- *(xen)* dynamic platform architecture ([#194](https://github.com/edera-dev/krata/pull/194))

### Fixed
- *(oci)* remove file size limit ([#142](https://github.com/edera-dev/krata/pull/142))
- *(oci)* use mirror.gcr.io as a mirror to docker hub ([#141](https://github.com/edera-dev/krata/pull/141))

### Other
- first pass of krata as an isolation engine
- *(xen)* split platform support into separate crate ([#195](https://github.com/edera-dev/krata/pull/195))
- *(xen)* move device creation into transaction interface ([#196](https://github.com/edera-dev/krata/pull/196))

## [0.0.10](https://github.com/edera-dev/krata/compare/v0.0.9...v0.0.10) - 2024-04-22

### Added
- implement guest exec ([#107](https://github.com/edera-dev/krata/pull/107))
- implement kernel / initrd oci image support ([#103](https://github.com/edera-dev/krata/pull/103))
- idm v2 ([#102](https://github.com/edera-dev/krata/pull/102))
- oci concurrency improvements ([#95](https://github.com/edera-dev/krata/pull/95))
- oci tar format, bit-perfect disk storage for config and manifest, concurrent image pulls ([#88](https://github.com/edera-dev/krata/pull/88))

### Fixed
- oci cache store should fallback to copy when rename won't work ([#96](https://github.com/edera-dev/krata/pull/96))

### Other
- update Cargo.lock dependencies

## [0.0.9](https://github.com/edera-dev/krata/compare/v0.0.8...v0.0.9) - 2024-04-15

### Added
- oci compliance work ([#85](https://github.com/edera-dev/krata/pull/85))
- oci packer can now use mksquashfs if available ([#70](https://github.com/edera-dev/krata/pull/70))
- basic kratactl top command ([#72](https://github.com/edera-dev/krata/pull/72))
- idm snooping ([#71](https://github.com/edera-dev/krata/pull/71))
- implement oci image progress ([#64](https://github.com/edera-dev/krata/pull/64))
- guest metrics support ([#46](https://github.com/edera-dev/krata/pull/46))

### Other
- init: default to xterm if TERM is not set ([#52](https://github.com/edera-dev/krata/pull/52))
- update Cargo.toml dependencies

## [0.0.8](https://github.com/edera-dev/krata/compare/v0.0.7...v0.0.8) - 2024-04-09

### Other
- update Cargo.lock dependencies

## [0.0.7](https://github.com/edera-dev/krata/compare/v0.0.6...v0.0.7) - 2024-04-09

### Other
- update Cargo.toml dependencies
- update Cargo.lock dependencies

## [0.0.6](https://github.com/edera-dev/krata/compare/v0.0.5...v0.0.6) - 2024-04-09

### Fixed
- increase channel acquisition timeout to support lower performance hosts ([#36](https://github.com/edera-dev/krata/pull/36))

### Other
- update Cargo.toml dependencies
- update Cargo.lock dependencies

## [0.0.5](https://github.com/edera-dev/krata/compare/v0.0.4...v0.0.5) - 2024-04-09

### Added
- *(ctl)* add help and about to commands and arguments ([#25](https://github.com/edera-dev/krata/pull/25))

### Other
- update Cargo.toml dependencies
- update Cargo.lock dependencies

## [0.0.4](https://github.com/edera-dev/krata/releases/tag/v${version}) - 2024-04-03

### Other
- implement automatic releases
- reimplement console to utilize channels, and provide logs support
- set hostname from launch config
- implement event stream retries
- work on parallel reconciliation
- implement parallel guest reconciliation
- log when a guest start failures occurs
- remove device restriction
- setup loopback interface
- place running tasks in cgroup
