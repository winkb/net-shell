#!/bin/bash

echo "Getting system information..."
echo "OS Version: $(uname -s) $(uname -r)"
echo "Hostname: $(hostname)"
echo "Current user: $(whoami)"
echo "System uptime: $(uptime)" 
echo "foo== {{ foo }}"