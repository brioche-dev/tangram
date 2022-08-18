#!/usr/bin/env bash
set -euo pipefail
shopt -s inherit_errexit

exec socat -,echo=0,icanon=0 "unix-connect:$1"
