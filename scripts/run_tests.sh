#!/bin/sh

TEST_SUITE=${1:-all}

print_help() {
    echo "Usage: $0 [<test_suite>] | $0 [help|-h|--help]"
    echo "Available Test Suites:"
    echo "-   all (or no argument) : run all test suites"
    echo "-            constraints : run constraints tests"
    echo "-                   time : run time tests (~100s runtime)"
    echo "-             regression : run regression tests (~17m runtime)"
    echo ""
    echo "-                   full : run all test suites + random-stress + tasksets-rt-app"
    echo "- ---------------------- : excluded tests from the all command -------------"
    echo "-          random-stress : run randomly generated stress tests (~1h runtime)"
    echo "-               tasksets : run taskset tests - periodic-thread backend"
    echo "-        tasksets-rt-app : run taskset tests - rt-app backend"
}

setup() {
    echo "* Preliminary Setup *"
    (
        ./test_suite_v2/tools mount-cgroup-fs       &&
        ./test_suite_v2/tools move-to-root          &&
        ./test_suite_v2/tools mount-cgroup-cpu      &&
        ./test_suite_v2/tools mount-debug-fs        &&
        ./test_suite_v2/tools cgroup-setup -r 900
    ) || exit 1
}

constraints() {
    echo "* Constraints Tests *"
    ./test_suite_v2/constraints_cgroup_setup
}

time_tests() {
    echo "* Time Tests *"
    BATCH_TEST_CUSTOM_NAME="one-task-one-cpu" \
        ./test_suite_v2/time many -r 40 -p 100 --cpu-set 0 -t 10
    BATCH_TEST_CUSTOM_NAME="one-task-two-cpus" \
        ./test_suite_v2/time many -r 30 -p 100 --cpu-set 0-1 -t 10
    BATCH_TEST_CUSTOM_NAME="one-task-four-cpus" \
        ./test_suite_v2/time many -r 20 -p 100 --cpu-set 0-3 -t 10
    BATCH_TEST_CUSTOM_NAME="one-task-eight-cpus" \
        ./test_suite_v2/time many -r 10 -p 100 --cpu-set 0-7 -t 10
    BATCH_TEST_CUSTOM_NAME="one-task-all-cpus" \
        ./test_suite_v2/time many -r 5 -p 100 -t 10

    BATCH_TEST_CUSTOM_NAME="five-tasks-one-cpu" \
        ./test_suite_v2/time many -n 5 -r 40 -p 100 --cpu-set 0 -t 10
    BATCH_TEST_CUSTOM_NAME="five-tasks-two-cpus" \
        ./test_suite_v2/time many -n 5 -r 30 -p 100 --cpu-set 0-1 -t 10
    BATCH_TEST_CUSTOM_NAME="five-tasks-four-cpus" \
        ./test_suite_v2/time many -n 5 -r 20 -p 100 --cpu-set 0-3 -t 10
    BATCH_TEST_CUSTOM_NAME="five-tasks-eight-cpus" \
        ./test_suite_v2/time many -n 5 -r 10 -p 100 --cpu-set 0-7 -t 10
    BATCH_TEST_CUSTOM_NAME="five-tasks-all-cpus" \
        ./test_suite_v2/time many -n 5 -r 5 -p 100 -t 10
}

regression() {
    echo "* Known Regression Tests *"
    TESTBINDIR=test_suite_v2 ./test_suite_v2/regression fair-server -t 60
    TESTBINDIR=test_suite_v2 ./test_suite_v2/regression fifo -r 10 -p 100 -t 60
    TESTBINDIR=test_suite_v2 ./test_suite_v2/regression fifo -r 50 -p 100 -t 60
    TESTBINDIR=test_suite_v2 ./test_suite_v2/regression fifo -r 90 -p 100 -t 60
    TESTBINDIR=test_suite_v2 ./test_suite_v2/regression deadline -r 10 -p 100 -t 60
    TESTBINDIR=test_suite_v2 ./test_suite_v2/regression deadline -r 20 -p 100 -t 60
    TESTBINDIR=test_suite_v2 ./test_suite_v2/regression deadline -r 30 -p 100 -t 60
    BATCH_TEST_CUSTOM_NAME="migration-regression" \
        ./test_suite_v2/stress task-migration -r 1 -p 100 -P 0.1 -t 300
    BATCH_TEST_CUSTOM_NAME="affinity-regression" \
        ./test_suite_v2/stress task-pinning -r 1 -p 100 -P 0.1 --cpu-set1 0 --cpu-set2 1 -t 300
}

random_stress() {
    echo "* Random Stress Tests *"
    ./test_suite_v2/stress all -n 60 -t 5 --seed 42
    ./test_suite_v2/stress all -n 10 -t 300 --seed 4242
}

tasksets() {
    echo "* Taskset Tests - periodic-thread *"
    TESTBINDIR=bin ./test_suite_v2/taskset --runner periodic-thread all -n $(nproc) -i ./tasksets -o ./tasksets_out || true
}

tasksets_rt_app() {
    echo "* Taskset Tests - rt-app *"
    TESTBINDIR=bin ./test_suite_v2/taskset --runner rt-app all -n $(nproc) -i ./tasksets -o ./tasksets_out || true
}

export BATCH_TEST=1
if command -v tput >/dev/null 2>&1 && [ $(tput colors) -gt 0 ]; then
    export TERM_COLORS=1
fi

if [ $TEST_SUITE = "all" ]; then
    echo "*** Running all tests ***"
    setup
    constraints
    time_tests
    regression
elif [ $TEST_SUITE = "full" ]; then
    echo "*** Running all tests + excluded ones ***"
    setup
    constraints
    time_tests
    regression
    random_stress
    tasksets_rt_app
elif [ $TEST_SUITE = "help" ] || [ $TEST_SUITE = "-h" ] || [ $TEST_SUITE = "--help" ]; then
    print_help
elif [ $TEST_SUITE = "constraints" ]; then
    setup
    constraints
elif [ $TEST_SUITE = "time" ]; then
    setup
    time_tests
elif [ $TEST_SUITE = "regression" ]; then
    setup
    regression
elif [ $TEST_SUITE = "random-stress" ]; then
    setup
    random_stress
elif [ $TEST_SUITE = "tasksets" ]; then
    setup
    tasksets
elif [ $TEST_SUITE = "tasksets-rt-app" ]; then
    setup
    tasksets_rt_app
else
    echo "Unknown test suite: $TEST_SUITE"
    print_help
fi