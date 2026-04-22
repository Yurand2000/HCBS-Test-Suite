#!/bin/sh

runtime=10000
period=100000
self=$$

# setup cgroup
if [ ! -d "/sys/fs/cgroup/g0" ]; then
    mkdir -p /sys/fs/cgroup/g0 || return 1
fi

old_period=$(cat /sys/fs/cgroup/g0/cpu.rt_period_us)
if [ $old_period -gt 0 ]; then
    echo 0 > /sys/fs/cgroup/g0/cpu.rt_runtime_us || return 1
fi
echo $period > /sys/fs/cgroup/g0/cpu.rt_period_us || return 1
echo $runtime > /sys/fs/cgroup/g0/cpu.rt_runtime_us || return 1

# set shell sched_fifo
chrt -p 99 $self || return 1

# start yes
yes < /dev/zero > /dev/null 2> /dev/null &
pid=$!

# make yes fifo and migrate it
echo $pid > /sys/fs/cgroup/g0/cgroup.procs || return 1
chrt -p 98 $pid || return 1
# echo $pid > /sys/fs/cgroup/g0/cgroup.procs || return 1

# wait 5 seconds and kill it
# sleep 5
kill $pid

# migrate the shell out and destroy the cgroup
chrt -o -p 0 $self || return 1
rmdir /sys/fs/cgroup/g0