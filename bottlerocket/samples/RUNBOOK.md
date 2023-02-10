# Bottlerocket Test System Sample Files

These files contain templates that can be populated and run using `cargo make` commands.

Examples of how to populate and run each one of these files can be found below.

Before running a `cargo make` command, several environment variables can be set to customize the generated file.

The directory from which to use the sample file corresponds with the value in `CLUSTER_TYPE`.

The files in [eks](./eks) are meant to be run on an EKS management cluster.
You can create a new cluster using the [eksctl](https://eksctl.io/introduction/) tool.

The files in [kind](./kind) are meant to be run on a `kind` cluster.
Directions on how to use a `kind` cluster with TestSys can be found in our [QUICKSTART](../../docs/QUICKSTART.md).

A list of the customizable variables can be found at the top of [Makefile.toml](./Makefile.toml).

_Note_: `ASSUME_ROLE` can be assigned the ARN of an AWS IAM role that should be used for all AWS calls.

## Generating a test file

A test file can be generated by executing a `cargo make create <sample-name>` command.
The following example will create an ECS test file.

```bash
cargo make create ecs-test
```

The file will be created in the `output` folder and the relative path to the generated file will be printed to stdout.

## Running a test file

A test file can be run by executing a `cargo make run <sample-name>` command.
The following example will run an ECS test.

```bash
cargo make run ecs-test
```

This command will generate the test file first, then run it on a Testsys cluster.
The status can be checked using `cli status`.

_Note_: `cargo make run-samples` can be used to run all samples in the directory specified by `CLUSTER_TYPE`.
This command cannot accept user-defined values for `CLUSTER_NAME`, `GPU`, `METADATA_URL`, `OVA_NAME`, `BOTTLEROCKET_AMI_ID`, `INSTANCE_TYPES`, or `VARIANT`.

## Cleaning the output folder

The output folder can be cleaned by executing `cargo make clean-output`.

## Special considerations

### Workload tests

When creating or running a workload test file, the variables `WORKLOAD_TEST_NAME` and `WORKLOAD_TEST_IMAGE_URI` need to be set by the user.

_Note_: An example of a workload test that can be used with the `ecs-workload-agent` or `k8s-workload-agent` is the `nvidia-smoke` test.
See the `nvidia-smoke` [README.md](../tests/workload/nvidia-smoke/README.md) for instructions on how to build and use its image.

### VMware tests

In order to create or run a VMware test, it is assumed that your vSphere config file has been sourced.
Specifically, the variables `GOVC_USERNAME`, `GOVC_PASSWORD`, `GOVC_DATACENTER`, `GOVC_DATASTORE`, `GOVC_URL`, `GOVC_NETWORK`, `GOVC_RESOURCE_POOL`, and `GOVC_FOLDER` need to be populated.

Additionally, `CONTROL_PLANE_ENDPOINT_IP` should contain the IP that the control plane should be identified by and `MGMT_CLUSTER_KUBECONFIG_PATH` should contain the absolute path to the file that contains the `kubeconfig` for the management cluster.