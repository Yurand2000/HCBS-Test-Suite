#!/bin/sh

sh /root/trace/start_trace.sh &

sleep 1

/root/test_suite/time uni -t 10 -n 5 -r 40 -p 100 --cpu-set 0