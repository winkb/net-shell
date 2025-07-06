#!/bin/bash

echo "=== Complex Extraction Demo ==="

# 模拟复杂的JSON输出
echo '{"app": {"name": "myapp", "version": "2.1.3", "config": {"debug": true}}}'

# 模拟多行进程信息
echo "Process: nginx (PID: 12345, Status: running)"
echo "Process: redis (PID: 67890, Status: active)"

# 模拟嵌套数据结构
echo '{"data": {"level1": {"level2": {"value": "extracted_value"}}}}'

# 模拟简单提取
echo "Simple: basic_value"

# 模拟更复杂的场景：从日志中提取时间戳
echo "2025-07-06 10:30:45 [INFO] User login successful"
echo "2025-07-06 10:31:12 [ERROR] Database connection failed"

echo "=== Demo completed ===" 