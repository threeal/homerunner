#!/bin/bash

# Refreshes the apt package index unless this script did so in the last hour.
# A failed update is ignored, so a mirror outage doesn't fail the job, and it
# leaves the stamp alone, so the next job tries again.
stamp=/var/lib/apt/periodic/update-success-stamp
if [ -z "$(find "$stamp" -mmin -60 2>/dev/null)" ]; then
  sudo apt-get update --error-on=any && sudo touch "$stamp" || true
fi
