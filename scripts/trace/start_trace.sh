#!/bin/sh

chrt -f 99 tracebox --txt -c /root/trace/sched-trace.cfg \
    -o /tmp/host0/trace-$(date -Iseconds).perfetto-trace