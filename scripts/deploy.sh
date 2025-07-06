#!/bin/bash

APP_NAME=$1
VERSION=$2

echo "Deploying application: $APP_NAME version $VERSION"
echo "Creating deployment directory..."
mkdir -p /tmp/deployments/$APP_NAME-$VERSION
echo "Deployed to: /tmp/deployments/$APP_NAME-$VERSION"
echo "Status: SUCCESS"
echo "Deployment completed at $(date)" 