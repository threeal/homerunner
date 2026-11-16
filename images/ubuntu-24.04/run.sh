#!/bin/bash

# Runs the runner, exiting with code 143 when stopped by a signal. The runner
# exits with code 0 both when it gives up, such as when its registration is
# gone, and when it's stopped, so with `--restart on-failure` this keeps a
# runner that gave up from restarting while still bringing a stopped one back
# when the Docker daemon restarts, as on a host reboot.
set -m
trap 'stopped=1; kill -TERM "$pid" 2>/dev/null' TERM INT
./run.sh "$@" &
pid=$!
while kill -0 "$pid" 2>/dev/null; do
  wait "$pid"
  status=$?
done
if [ -n "$stopped" ]; then
  exit 143
fi
exit "$status"
