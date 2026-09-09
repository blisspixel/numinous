#!/usr/bin/env bash
# Install the Linux headers the App and CLI audio and input stacks need.
#
# GitHub's ubuntu-latest image ships a Google Chrome apt source this
# repository does not use. A hash mismatch there has failed otherwise-green
# jobs, so the source is dropped before update.
set -euo pipefail
sudo rm -f \
  /etc/apt/sources.list.d/google-chrome.list \
  /etc/apt/sources.list.d/google-chrome.sources \
  /etc/apt/sources.list.d/google.list
sudo apt-get update
sudo apt-get install -y "$@"
