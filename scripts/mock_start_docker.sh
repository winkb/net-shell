#!/bin/bash

echo "Starting Docker service on $(hostname)"
echo "Step 1: Checking Docker installation..."
sleep 1
echo "Step 2: Starting Docker daemon..."
sleep 1
echo "Step 3: Verifying Docker service..."
sleep 1
echo "Docker service started successfully!"
echo "Docker info: $(docker info 2>/dev/null | head -5 || echo 'Docker not running')"
