#!/bin/sh

/root/test_suite/tools cgroup-setup -r 850
/root/test_suite/tools mount-debug-fs || exit 1
sh /root/utils/mount_virtfs.sh host0 /tmp/host0
sh /root/utils/mount_virtfs.sh build /tmp/build

echo 0 > /sys/kernel/debug/sched/fair_server/cpu0/runtime
echo 0 > /sys/kernel/debug/sched/fair_server/cpu1/runtime
echo 0 > /sys/kernel/debug/sched/fair_server/cpu2/runtime
echo 0 > /sys/kernel/debug/sched/fair_server/cpu3/runtime