#!/bin/sh
[ "$1" = --version ] && { echo "Faraday desktop app 0.2.0"; exit 0; }
printf started > "$0.mark"
