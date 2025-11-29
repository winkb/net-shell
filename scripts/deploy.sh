#!/bin/bash

APP_NAME="{{ app_name }}"
VERSION="{{ os_version }}"


echo "master ip: {{ master_ip }} os_version: {{ os_version }}, os_version_num: {{ os_version_num }} "

echo "Deploying application: $APP_NAME version $VERSION"
echo "Creating deployment directory..."
mkdir -p /tmp/deployments/$APP_NAME
echo "Deployed to: /tmp/deployments/$APP_NAME"
echo "Status: SUCCESS"
echo "Deployment completed at $(date)" 
