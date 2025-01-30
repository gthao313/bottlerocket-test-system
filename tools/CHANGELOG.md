# Changelog

This changelog documents changes to the `bottlerocket-test-tools` container image.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Since this project is only a vessel for packaging a few binary tools, its adherence to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) is loose at best.

## [0.10.0] - 2025-01-30

Update Bottlerocket SDK to 0.50.1
Update eksctl to 0.202.0

## [0.9.0] - 2024-07-23

Update Bottlerocket SDK to 0.42.0
Update kubernetes to EKSD 1.27.35
Update eksctl to 0.187.0
Update helm to 3.15.3
Update aws-iam-authenticator to 0.6.21
Update sonobuoy to 0.57.1

## [0.8.0] - 2024-01-29

Update eksctl to 0.169.0
Update helm to 3.14.0
Update Bottlerocket SDK to 0.37.0

## [0.7.0] - 2023-09-13

Removed eksctl-anywhere
Update Bottlerocket SDK used to build the tools iamge to v0.34.1

### Contents

- eksctl 0.144.0
- sonobuoy v0.56.15

## [0.6.0] - 2023-06-12

Update eksctl, eksctl-anywhere, and sonobuoy

### Contents

- eksctl 0.144.0
- eksctl-anywhere 0.16.0
- sonobuoy v0.56.15

## [0.5.0] - 2023-04-19

Update eksctl, eksctl-anywhere, kubernetes, aws-iam-authenticator
Add helm

### Contents

- eksctl 0.136.0
- kubeadm v1.26.3
- sonobuoy v0.56.4
- eksctl-anywhere 0.15.1-35
- helm 3.8.2
- aws-iam-authenticator 0.6.8

## [0.4.0] - 2023-03-03

Update eksctl, eksctl-anywhere

### Contents

- eksctl 0.132.0
- kubeadm v1.23.13
- sonobuoy v0.56.4
- eksctl-anywhere 0.14.3-30

## [0.3.0] - 2022-12-15

Update eksctl, eksctl-anywhere

### Contents

- eksctl 0.120.0
- kubeadm v1.23.13
- sonobuoy v0.56.4
- eksctl-anywhere 0.12.4-24

## [0.2.1] - 2022-11-01

Update bottlerocket sdk to v0.28.0

## [0.2.0] - 2022-10-24

Update eksctl, sonobuoy, kubeadm
Add kubectl, eksctl-anywhere

### Contents

- boringtun v0.4.0 was removed
- eksctl 0.112.0
- kubeadm v1.23.13
- sonobuoy v0.56.4
- eksctl-anywhere 0.11.4-21

## [0.1.0] - 2022-05-11

Initial version, x86_64-only for now.

### Contents

- boringtun v0.4.0
- eksctl 0.82.0
- kubeadm v1.21.6
- sonobuoy v0.53.2

<!-- example comparison for future releases
[0.2.0]: https://github.com/bottlerocket-os/bottlerocket-test-system/compare/tools-v0.1.0...tools-v0.2.0 -->

[0.2.0]: https://github.com/bottlerocket-os/bottlerocket-test-system/compare/tools-v0.1.0...tools-v0.2.0
[0.1.0]: https://github.com/bottlerocket-os/bottlerocket-test-system/tree/tools-v0.1.0
