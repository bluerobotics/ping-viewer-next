#!/bin/bash
set -m
echo "Starting ping viewer next..."
cd app
su pingviewer -c "./ping-viewer-next --enable-auto-create --rest-server 0.0.0.0:6060"
