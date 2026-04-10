#!/bin/sh

sh /root/trace/start_trace.sh &

sleep 1

/root/test_suite/time multi -t 10 -C 40/100/0-1