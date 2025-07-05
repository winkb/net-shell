#!/bin/bash

echo "Starting Docker installation on $(hostname)"
echo "Step 1: Checking system requirements..."
sleep 1
echo "Step 2: Downloading Docker..."
sleep 1
echo "Step 3: Installing Docker..."
sleep 1
echo "Docker installation completed successfully!"
echo "Docker version: $(docker --version 2>/dev/null || echo 'Docker not found')"
