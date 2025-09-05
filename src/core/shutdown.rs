//! 关机执行器模块
//! 
//! 负责执行系统关机操作，支持多种关机方式和错误处理

use anyhow::{Result, anyhow};
use log::{info, warn, error};
use tokio::process::Command as AsyncCommand;

use crate::core::types::{ShutdownMethod, UserPermissions};
use crate::core::system_compat::SystemCompatibility;

#[cfg(windows)]
use winapi::um::winuser::{ExitWindowsEx, EWX_SHUTDOWN, EWX_FORCE};
#[cfg(windows)]
use winapi::um::winnt::{TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY};
#[cfg(windows)]
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
#[cfg(windows)]
use winapi::um::securitybaseapi::AdjustTokenPrivileges;
#[cfg(windows)]
use winapi::um::winbase::LookupPrivilegeValueW;
#[cfg(windows)]
use winapi::shared::minwindef::FALSE;
use winapi::um::winnt::LUID;
#[cfg(windows)]
use winapi::um::winnt::{HANDLE, TOKEN_PRIVILEGES, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED};

/// 关机执行器
#[derive(Debug)]
pub struct ShutdownExecutor {
    /// 首选关机方法
    preferred_method: ShutdownMethod,
    /// 系统兼容性信息
    system_compatibility: SystemCompatibility,
    /// 用户权限信息
    user_permissions: UserPermissions,
}

impl ShutdownExecutor {
    /// 创建新的关机执行器
    pub async fn new() -> Result<Self> {
        // 初始化系统兼容性检查器
        let mut system_compatibility = SystemCompatibility::new();
        system_compatibility.initialize().await?;
        let user_permissions = Self::check_user_permissions()?;
        
        // 根据系统版本和权限选择最佳关机方法
        let preferred_method = if user_permissions.can_shutdown {
            if let Some(version) = system_compatibility.get_windows_version() {
                if version.major >= 10 {
                    ShutdownMethod::WinAPI
                } else {
                    ShutdownMethod::Command
                }
            } else {
                ShutdownMethod::Command
            }
        } else {
            ShutdownMethod::Command
        };
        
        info!("关机执行器初始化完成，首选方法: {:?}", preferred_method);
        
        Ok(Self {
            preferred_method,
            system_compatibility,
            user_permissions,
        })
    }
    
    /// 执行关机操作
    /// 
    /// # 参数
    /// 
    /// * `delay_seconds` - 延迟秒数（0表示立即关机）
    pub async fn shutdown(&self, delay_seconds: u32) -> Result<()> {
        info!("开始执行关机操作，延迟: {}秒", delay_seconds);
        
        // 尝试首选方法
        match self.try_shutdown(self.preferred_method, delay_seconds).await {
            Ok(_) => {
                info!("关机命令执行成功");
                Ok(())
            },
            Err(e) => {
                warn!("首选关机方法失败: {}, 尝试备用方法", e);
                
                // 尝试备用方法
                let backup_method = match self.preferred_method {
                    ShutdownMethod::WinAPI => ShutdownMethod::Command,
                    ShutdownMethod::Command => ShutdownMethod::WinAPI,
                };
                
                self.try_shutdown(backup_method, delay_seconds).await
                    .map_err(|backup_err| {
                        error!("所有关机方法都失败了");
                        anyhow!("关机失败: 首选方法错误: {}, 备用方法错误: {}", e, backup_err)
                    })
            }
        }
    }
    
    /// 取消关机操作
    /// 
    /// 尝试取消之前设置的延迟关机
    pub async fn cancel_shutdown(&self) -> Result<()> {
        info!("尝试取消关机操作");
        
        // 使用shutdown命令取消
        let output = AsyncCommand::new("shutdown")
            .args(["/a"])
            .output()
            .await
            .map_err(|e| anyhow!("执行取消关机命令失败: {}", e))?;
            
        if output.status.success() {
            info!("关机操作已取消");
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("取消关机失败: {}", error_msg))
        }
    }
    
    /// 检查是否有待执行的关机任务
    pub async fn has_pending_shutdown(&self) -> bool {
        // 通过查询系统状态检查是否有待执行的关机
        // 这里简化实现，实际可以通过查询系统服务或注册表
        false
    }
    
    /// 尝试使用指定方法关机
    async fn try_shutdown(&self, method: ShutdownMethod, delay_seconds: u32) -> Result<()> {
        match method {
            ShutdownMethod::Command => self.shutdown_by_command(delay_seconds).await,
            ShutdownMethod::WinAPI => self.shutdown_by_winapi(delay_seconds).await,
        }
    }
    
    /// 使用shutdown.exe命令关机
    async fn shutdown_by_command(&self, delay_seconds: u32) -> Result<()> {
        info!("使用shutdown命令关机");
        
        let mut cmd = AsyncCommand::new("shutdown");
        cmd.args(["/s", "/f"]); // /s = 关机, /f = 强制关闭应用程序
        
        if delay_seconds > 0 {
            cmd.args(["/t", &delay_seconds.to_string()]);
        } else {
            cmd.args(["/t", "0"]);
        }
        
        // 添加关机消息
        cmd.args(["/c", "QtShut 定时关机"]);
        
        let output = cmd.output().await
            .map_err(|e| anyhow!("执行shutdown命令失败: {}", e))?;
            
        if output.status.success() {
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("shutdown命令执行失败: {}", error_msg))
        }
    }
    
    /// 使用Windows API关机
    #[cfg(windows)]
    async fn shutdown_by_winapi(&self, delay_seconds: u32) -> Result<()> {
        info!("使用Windows API关机");
        
        // 如果有延迟，先使用命令行方式设置延迟
        if delay_seconds > 0 {
            return self.shutdown_by_command(delay_seconds).await;
        }
        
        // 获取关机权限
        self.enable_shutdown_privilege()?;
        
        // 执行关机
        let result = unsafe {
            ExitWindowsEx(EWX_SHUTDOWN | EWX_FORCE, 0)
        };
        
        if result != 0 {
            Ok(())
        } else {
            Err(anyhow!("Windows API关机失败"))
        }
    }
    
    /// 非Windows系统的API关机实现
    #[cfg(not(windows))]
    async fn shutdown_by_winapi(&self, _delay_seconds: u32) -> Result<()> {
        Err(anyhow!("Windows API在非Windows系统上不可用"))
    }
    
    /// 启用关机权限
    #[cfg(windows)]
    fn enable_shutdown_privilege(&self) -> Result<()> {
        unsafe {
            let mut token_handle: HANDLE = std::ptr::null_mut();
            
            // 打开进程令牌
            let result = OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token_handle,
            );
            
            if result == FALSE {
                return Err(anyhow!("无法打开进程令牌"));
            }
            
            // 查找关机权限
            let mut luid = LUID { LowPart: 0, HighPart: 0 };
            let privilege_name = "SeShutdownPrivilege\0".encode_utf16().collect::<Vec<u16>>();
            
            let result = LookupPrivilegeValueW(
                std::ptr::null(),
                privilege_name.as_ptr(),
                &mut luid,
            );
            
            if result == FALSE {
                return Err(anyhow!("无法查找关机权限"));
            }
            
            // 调整令牌权限
            let mut token_privileges = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            
            let result = AdjustTokenPrivileges(
                token_handle,
                FALSE,
                &mut token_privileges,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            
            if result == FALSE {
                return Err(anyhow!("无法调整令牌权限"));
            }
            
            Ok(())
        }
    }
    
    /// 检查用户权限
    fn check_user_permissions() -> Result<UserPermissions> {
        // 简化的权限检查实现
        // 实际应用中可以通过Windows API检查用户是否有关机权限
        
        #[cfg(windows)]
        {
            // 在Windows上，大多数用户都有关机权限
            Ok(UserPermissions {
                can_shutdown: true,
                is_admin: Self::is_admin(),
            })
        }
        
        #[cfg(not(windows))]
        {
            Ok(UserPermissions {
                can_shutdown: false,
                is_admin: false,
            })
        }
    }
    
    /// 检查是否为管理员权限
    #[cfg(windows)]
    fn is_admin() -> bool {
        // 简化实现，实际可以通过Windows API检查
        // 这里假设普通用户也可以关机
        false
    }
    
    #[cfg(not(windows))]
    fn is_admin() -> bool {
        false
    }
    
    /// 验证关机命令是否可用
    pub async fn validate_shutdown_capability(&self) -> Result<()> {
        info!("验证关机功能可用性");
        
        // 测试shutdown命令是否存在
        let output = AsyncCommand::new("shutdown")
            .args(["/?"])
            .output()
            .await;
            
        match output {
            Ok(result) if result.status.success() => {
                info!("shutdown命令可用");
                Ok(())
            },
            Ok(_) => {
                warn!("shutdown命令存在但可能无法正常工作");
                Ok(())
            },
            Err(e) => {
                error!("shutdown命令不可用: {}", e);
                Err(anyhow!("系统不支持shutdown命令: {}", e))
            }
        }
    }
    
    /// 获取关机方法信息
    pub fn get_shutdown_info(&self) -> String {
        format!(
            "首选方法: {:?}, 用户权限: 可关机={}, 管理员={}",
            self.preferred_method,
            self.user_permissions.can_shutdown,
            self.user_permissions.is_admin
        )
    }
    
    /// 模拟关机（用于测试）
    /// 
    /// 在测试环境中使用，不会真正关机
    #[cfg(test)]
    pub async fn simulate_shutdown(&self, delay_seconds: u32) -> Result<()> {
        info!("模拟关机操作，延迟: {}秒", delay_seconds);
        
        if delay_seconds > 0 {
            use std::time::Duration;

            tokio::time::sleep(Duration::from_secs(delay_seconds as u64)).await;
        }
        
        info!("模拟关机完成");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_shutdown_executor_creation() {
        let executor = ShutdownExecutor::new().await;
        assert!(executor.is_ok());
    }
    
    #[tokio::test]
    async fn test_validate_shutdown_capability() {
        let executor = ShutdownExecutor::new().await.unwrap();
        
        // 在测试环境中，这可能会失败，这是正常的
        let _ = executor.validate_shutdown_capability().await;
    }
    
    #[tokio::test]
    async fn test_simulate_shutdown() {
        let executor = ShutdownExecutor::new().await.unwrap();
        
        // 测试模拟关机
        let result = executor.simulate_shutdown(0).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_get_shutdown_info() {
        let executor = ShutdownExecutor::new().await.unwrap();
        
        let info = executor.get_shutdown_info();
        assert!(!info.is_empty());
    }
}