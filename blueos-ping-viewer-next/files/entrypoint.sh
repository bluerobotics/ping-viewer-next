#!/bin/bash
set -m
echo "Starting ping viewer next..."
cd app
mkdir logs
mkdir recordings
chmod -R 755 /app/logs
chmod -R 755 /app/recordings
chown -R pingviewer:pingviewer /app/logs
chown -R pingviewer:pingviewer /app/recordings
su pingviewer -c "./ping-viewer-next --enable-auto-create --rest-server 0.0.0.0:6060"
