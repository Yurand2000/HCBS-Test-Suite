# HCBS-Test-Suite

Rust-based tests for [Hierarchical Constant Bandwidth Server](https://github.com/Yurand2000/HCBS-patch)

## 🚀 Quick Start

### Prerequisites

- **Make**
- **Git**
- **GCC**
- **Cmake**
- **Rust**: ≥1.88.0

### Installation

The building process is based on Make, and will produce a variety of targets:
- Install the test software in a target directory.
- Create a tar.gz archive of the test software.
- Create a initramfs, containing the test software + BusyBox and Sudo, to run the kernel with QEMU.

```bash
# The all target will build both the install-tar and initramfs targets.
> make [all]

# Build test software
> make build

# Install the test software in the specified directory
# if the output folder is not specified, it defaults to ./install
> make O=<install directory> install

# Build and pack togheter in a tar compressed archive (<BUILD_DIR>/install.tar.gz)
> make install-tar

# Create an initramfs for QEMU (<BUILD_DIR>/core.gz)
# the initramfs contains BusyBox, Sudo and the compiled test software
> make initramfs

# Clean build directory
> make clean

# Out-of-tree build (by default the build directory is ./build)
> make BUILD=<your build directory> [other commands]
```

### Basic Usage

The tests can be manually run one by one, useful for development and debugging, or may be run using the dedicated `run_tests.sh` script.

The following concerns the `run_tests.sh` script:

```sh
# Step 1
# Get a running kernel with the patchset applied.
# Make sure that you have a kernel with cgroups-v2 enabled.
# The script will setup the real-time cgorup system in a default way,
# make sure you don't have any SCHED_DEADLINE task active. Also, the
# script will migrate all SCHED_FIFO/SCHED_RR tasks in the root cgroup.

# Step 2
# Run all standard tests
> sh run_tests.sh [all]

# OR Run all standard tests + randomly generated tests
> sh run_tests.sh full

# ---------------- #
# Get available test suites
> sh run_tests.sh help

# Run a single test suite
> sh run_tests.sh <test suite>
```

### Advanced Usage

Tests executables can be run manually. They can be found at `test_suite_v2`. Just run any executable without argument to get the help screen. Take a look at section Available Tests for more information.

```bash
# Examples of manually run tests.
> ./test_suite_v2/regression fair-server -t 10

> ./test_suite_v2/stress migrate -r 10 -p 100 -P 0.1 -t 60

# There is also useful tools
> ./test_suite_v2/tools -h
Usage: tools <COMMAND>

Commands:
  hog              CPU hog
  mount-cgroup-fs  Mount CGroup filesystem
  mount-cgroup-cpu Mount CGroup filesystem and CPU controller
  mount-debug-fs   Mount DebugFS
  rt-bw-change     Change the global badwidth limits of real-time tasks
                   (both FIFO/RR and DEADLINE)
  move-to-root     Move all real-time tasks to the root control group
  cgroup-setup     Change the runtime and period to the given control group
  hrtick           Enable/Disable the HRTICK_DL scheduler feature
[...]
```

## 🛠️ Available Tests

### 1. Constraints

These tests assert that hard constraints, such as schedulability conditions, are respected, stressing corner cases and the reaching of illegal states.

### 2. Regression

Regression tests concern the compatibility of HCBS with already existing kernel features, such as fair-servers and SCHED_DEADLINE tasks.

### 3. Stress

Stress tests are designed to repeatedly invoke the scheduler in all the exposed interfaces (such as repeated changes in affinity or policy), to detect bugs and race conditions.

### 4. Time

These basic time tests are just a benchmark to assert that the HCBS mechanism works correctly, by starting a bunch of processes inside a cgroup and confirming that they consumed their expected amount of bandwidth.

### 5. Taskset

Taskset tests are more complex: given a set of (generated) periodic tasks and their bandwidth requirements, schedulability analyses are performed to decide whether or not a given hardware configuration can run the taskset. In particular, for each taskset, a HCBS's cgroup configuration along with the number of necessary CPUs is generated. These are mathematically guaranteed to be schedulable.

The next step of this test suite is to configure cgroups as computed and to run the taskset, to verify that the HCBS implementation works as intended and that the scheduling overheads are within reasonable bounds.

### A. Extras

#### 1. Taskset Generator

The **taskset_gen** executable can be used to generate a suite of syntetic tasksets to be used by the **taskset** executable.

#### 2. Tools

The extra **tools** executable exposes a number of QoL features to simplify the setup/use of HCBS and related features. Currently (2025-11-11) it provides:

- **CPU hog**
- **Mounting of the cgroup's filesystem**
- **Mounting of the cgroup's filesystem and enabling of the CPU controller**
- **Mounting of the debug filesystem**
- **Migration of all the SCHED_FIFO/SCHED_RR tasks to the root control group.** This is necessary to enable the CPU controller on the cgroups, as some Linux distributions start some rt-tasks in some cgroups before enabling the mechanism.
- **Change the global bandwidth limits of real-time tasks.**
- **Enable/Disable HRTick**
- **Set scheduler to SCHED_DEADLINE for the given process**, useful in case the default *chrt* does not support it.
- **HCBS-specific cgroup setup**

## 📄 License

This project is licensed under the GNU General Public License v3 - see the [LICENSE](LICENSE) file for details.

## 👤 Author

**HCBS-Test-Suite** was developed by:
- **Yuri Andriaccio** [yurand2000@gmail.com](mailto:yurand2000@gmail.com), [GitHub](https://github.com/Yurand2000).

## 📝 TODO - Future Work

- [ ] **More Tests**: The repository will be updated with new tests, as the HCBS patches continue to evolve.
- [X] **Rewrite Taskset Analyzer**: The taskset generation software is currently closed source, but I plan to rewrite it to include newer algorithms and make it open source.
- [ ] **Rewrite Periodic Task in Rust**: Currently the periodic_task and periodic_thread binaries come from another git repository and are written in C. This would remove the need for the C compiler (only non BusyBox builds).

---

**HCBS-Test-Suite**