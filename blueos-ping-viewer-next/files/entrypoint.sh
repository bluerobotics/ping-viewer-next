#!/bin/bash
set -meuo pipefail

echo "Starting ping viewer..."

cd /app

for dir in /app/logs /app/recordings /app/settings; do
    mkdir -p "$dir"
    chown -R pingviewer:pingviewer "$dir"
    chmod 755 "$dir"
done

su pingviewer -c "./ping-viewer-next --enable-auto-create --rest-server 0.0.0.0:6060"
