use anyhow::{Context, Result};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use tracing::info;

use crate::models::{ExecutionResult, SshConfig};

/// SSH执行器
pub struct SshExecutor;

impl SshExecutor {
    /// 通过SSH执行脚本
    pub fn execute_script(ssh_config: &SshConfig, script_path: &str) -> Result<ExecutionResult> {
        info!("Connecting to {}:{} as {}", ssh_config.host, ssh_config.port, ssh_config.username);

        // 读取脚本内容
        let script_content = std::fs::read_to_string(script_path)
            .context(format!("Failed to read script file: {}", script_path))?;

        // 建立TCP连接
        let tcp = TcpStream::connect(format!("{}:{}", ssh_config.host, ssh_config.port))
            .context("Failed to connect to SSH server")?;

        // 创建SSH会话
        let mut sess = Session::new()
            .context("Failed to create SSH session")?;
        
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .context("SSH handshake failed")?;

        // 认证
        if let Some(ref password) = ssh_config.password {
            sess.userauth_password(&ssh_config.username, password)
                .context("SSH password authentication failed")?;
        } else if let Some(ref key_path) = ssh_config.private_key_path {
            sess.userauth_pubkey_file(&ssh_config.username, None, Path::new(key_path), None)
                .context("SSH key authentication failed")?;
        } else {
            return Err(anyhow::anyhow!("No authentication method provided"));
        }

        info!("SSH authentication successful");

        // 执行命令
        let mut channel = sess.channel_session()
            .context("Failed to create SSH channel")?;

        channel.exec(&script_content)
            .context("Failed to execute command")?;

        // 读取输出
        let mut stdout = String::new();
        let mut stderr = String::new();

        channel.read_to_string(&mut stdout)
            .context("Failed to read stdout")?;

        channel.stderr().read_to_string(&mut stderr)
            .context("Failed to read stderr")?;

        channel.wait_close()
            .context("Failed to wait for channel close")?;

        let exit_code = channel.exit_status()
            .context("Failed to get exit status")?;

        info!("SSH command executed with exit code: {}", exit_code);

        Ok(ExecutionResult {
            success: exit_code == 0,
            stdout,
            stderr,
            script: script_path.to_string(),
            exit_code,
            execution_time_ms: 0, // 将在上层计算
            error_message: None,
        })
    }
} 