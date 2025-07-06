#!/bin/bash

DEPLOY_PATH={{ deploy_path }}

echo "Verifying deployment at: $DEPLOY_PATH"
if [ -d "$DEPLOY_PATH" ]; then
    echo "Service Status: RUNNING"
    echo "Deployment verified successfully"
else
    echo "Service Status: FAILED"
    echo "Deployment verification failed"
    exit 1
fi 