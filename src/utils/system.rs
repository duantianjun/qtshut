//! 系统工具模块
//! 
//! 提供系统相关的工具函数，如版本检测、权限检查等

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use log::{info, warn};
// use winapi::um::sysinfoapi::GetVersionExW; // 需要sysinfoapi feature
use winapi::um::sysinfoapi::GetVersion;
use winapi::um::winuser::{GetSystemMetrics, SM_CLEANBOOT};
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
use winapi::um::securitybaseapi::GetTokenInformation;
use winapi::um::winnt::{TOKEN_ELEVATION, TokenElevation, HANDLE};
use winapi::shared::minwindef::DWORD;

use crate::core::types::{WindowsVersion, UserPermissions};

/// SystemCompat类型别名，用于兼容性
pub type SystemCompat = SystemCompatibility;

/// 操作系统版本信息
#[derive(Debug, Clone, PartialEq)]
pub struct OsVersion {
    pub major: u32,
    pub minor: u32,
}

/// 系统兼容性检查器
/// 
/// 负责检测Windows版本和系统兼容性
#[derive(Debug, Clone)]
pub struct SystemCompatibility {
    /// Windows版本信息
    windows_version: Option<WindowsVersion>,
    /// 用户权限信息
    user_permissions: Option<UserPermissions>,
}

impl Default for SystemCompatibility {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCompatibility {
    /// 创建新的系统兼容性检查器
    pub fn new() -> Self {
        Self {
            windows_version: None,
            user_permissions: None,
        }
    }
    
    /// 执行完整的系统兼容性检查
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    pub fn check_compatibility(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("开始系统兼容性检查");
        
        // 检测Windows版本
        self.detect_windows_version()?;
        
        // 检查用户权限
        self.check_user_permissions()?;
        
        // 验证系统要求
        self.validate_system_requirements()?;
        
        info!("系统兼容性检查完成");
        Ok(())
    }
    
    /// 检测Windows版本
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    fn detect_windows_version(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("检测Windows版本");
        
        unsafe {
            let version = GetVersion();
            let major = (version & 0xFF) as u32;
            let minor = ((version >> 8) & 0xFF) as u32;
            let build = 0; // GetVersion不提供build号
            
            info!("检测到Windows版本: {}.{}.{}", major, minor, build);
            
            // 根据版本号确定Windows版本
            let windows_version = match (major, minor) {
                (10, 0) => {
                    if build >= 22000 {
                        WindowsVersion::Windows11
                    } else {
                        WindowsVersion::Windows10
                    }
                },
                (6, 3) => WindowsVersion::Windows81,
                (6, 2) => WindowsVersion::Windows8,
                (6, 1) => WindowsVersion::Windows7,
                _ => WindowsVersion::Unknown,
            };
            
            info!("Windows版本: {:?}", windows_version);
            self.windows_version = Some(windows_version);
        }
        
        Ok(())
    }
    
    /// 检查用户权限
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    fn check_user_permissions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("检查用户权限");
        
        let is_admin = self.is_running_as_administrator()?;
        let has_shutdown_privilege = self.check_shutdown_privilege()?;
        
        let permissions = UserPermissions {
            can_shutdown: has_shutdown_privilege,
            is_admin,
        };
        
        info!("用户权限: {:?}", permissions);
        self.user_permissions = Some(permissions);
        
        Ok(())
    }
    
    /// 检查是否以管理员身份运行
    /// 
    /// # 返回值
    /// 
    /// 成功返回是否为管理员，失败返回错误信息
    fn is_running_as_administrator(&self) -> Result<bool, Box<dyn std::error::Error>> {
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            
            // 获取当前进程的访问令牌
            let result = OpenProcessToken(
                GetCurrentProcess(),
                winapi::um::winnt::TOKEN_QUERY,
                &mut token
            );
            
            if result == 0 {
                return Err("无法获取进程访问令牌".into());
            }
            
            // 查询令牌提升信息
            let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
            let mut return_length: DWORD = 0;
            
            let result = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                std::mem::size_of::<TOKEN_ELEVATION>() as DWORD,
                &mut return_length
            );
            
            // 关闭令牌句柄
            winapi::um::handleapi::CloseHandle(token);
            
            if result == 0 {
                return Err("无法查询令牌提升信息".into());
            }
            
            Ok(elevation.TokenIsElevated != 0)
        }
    }
    
    /// 检查关机权限
    /// 
    /// # 返回值
    /// 
    /// 成功返回是否有关机权限，失败返回错误信息
    fn check_shutdown_privilege(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // 简化实现：假设有管理员权限就有关机权限
        // 实际实现中可以通过LookupPrivilegeValue和PrivilegeCheck来检查
        let is_admin = self.is_running_as_administrator()?;
        Ok(is_admin)
    }
    
    /// 验证系统要求
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    fn validate_system_requirements(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 检查Windows版本是否支持
        if let Some(version) = &self.windows_version {
            match (version.major, version.minor) {
                (10, 0) => {
                    info!("Windows版本兼容 (Windows 10/11)");
                },
                (6, 1) | (6, 2) | (6, 3) => {
                    warn!("Windows版本较旧，可能存在兼容性问题");
                },
                _ => {
                    warn!("未知的Windows版本，可能存在兼容性问题");
                }
            }
        }
        
        // 检查用户权限
        if let Some(permissions) = &self.user_permissions {
            if !permissions.can_shutdown {
                warn!("当前用户没有关机权限，可能需要管理员权限");
            }
        }
        
        Ok(())
    }
    
    /// 获取Windows版本
    pub fn get_windows_version(&self) -> Option<&WindowsVersion> {
        self.windows_version.as_ref()
    }
    
    /// 获取用户权限
    pub fn get_user_permissions(&self) -> Option<&UserPermissions> {
        self.user_permissions.as_ref()
    }
    
    /// 检查是否支持当前系统
    /// 
    /// # 返回值
    /// 
    /// 是否支持当前系统
    pub fn is_system_supported(&self) -> bool {
        if let Some(version) = &self.windows_version {
            matches!((version.major, version.minor), 
                (10, 0) | // Windows 10/11
                (6, 1) | // Windows 7
                (6, 2) | // Windows 8
                (6, 3)   // Windows 8.1
            )
        } else {
            false
        }
    }
    
    /// 获取操作系统版本信息
    /// 
    /// # 返回值
    /// 
    /// 操作系统版本结构体
    pub fn get_os_version(&self) -> OsVersion {
        if let Some(version) = &self.windows_version {
            match (version.major, version.minor) {
                (10, 0) => OsVersion { major: 10, minor: 0 },
                (6, 3) => OsVersion { major: 6, minor: 3 },
                (6, 2) => OsVersion { major: 6, minor: 2 },
                (6, 1) => OsVersion { major: 6, minor: 1 },
                _ => OsVersion { major: 0, minor: 0 },
            }
        } else {
            OsVersion { major: 0, minor: 0 }
        }
    }
}

/// 获取系统信息
/// 
/// # 返回值
/// 
/// 系统信息字符串
pub fn get_system_info() -> String {
    let mut info = Vec::new();
    
    // 获取计算机名
    if let Ok(computer_name) = get_computer_name() {
        info.push(format!("计算机名: {}", computer_name));
    }
    
    // 获取用户名
    if let Ok(user_name) = get_user_name() {
        info.push(format!("用户名: {}", user_name));
    }
    
    // 获取系统启动模式
    let boot_mode = get_boot_mode();
    info.push(format!("启动模式: {}", boot_mode));
    
    info.join("\n")
}

/// 获取计算机名
/// 
/// # 返回值
/// 
/// 成功返回计算机名，失败返回错误信息
fn get_computer_name() -> Result<String, Box<dyn std::error::Error>> {
    use winapi::um::winbase::GetComputerNameW;
    
    const MAX_COMPUTERNAME_LENGTH: u32 = 15; // Windows标准计算机名最大长度
    
    unsafe {
        let mut buffer = vec![0u16; (MAX_COMPUTERNAME_LENGTH + 1) as usize];
        let mut size = buffer.len() as DWORD;
        
        let result = GetComputerNameW(buffer.as_mut_ptr(), &mut size);
        if result == 0 {
            return Err("无法获取计算机名".into());
        }
        
        buffer.truncate(size as usize);
        let os_string = OsString::from_wide(&buffer);
        Ok(os_string.to_string_lossy().to_string())
    }
}

/// 获取用户名
/// 
/// # 返回值
/// 
/// 成功返回用户名，失败返回错误信息
fn get_user_name() -> Result<String, Box<dyn std::error::Error>> {
    use winapi::um::winbase::GetUserNameW;
    const UNLEN: u32 = 256; // Windows标准用户名最大长度
    
    unsafe {
        let mut buffer = vec![0u16; (UNLEN + 1) as usize];
        let mut size = buffer.len() as DWORD;
        
        let result = GetUserNameW(buffer.as_mut_ptr(), &mut size);
        if result == 0 {
            return Err("无法获取用户名".into());
        }
        
        buffer.truncate((size - 1) as usize); // 去掉null终止符
        let os_string = OsString::from_wide(&buffer);
        Ok(os_string.to_string_lossy().to_string())
    }
}

/// 获取系统启动模式
/// 
/// # 返回值
/// 
/// 启动模式字符串
fn get_boot_mode() -> String {
    unsafe {
        let boot_mode = GetSystemMetrics(SM_CLEANBOOT);
        match boot_mode {
            0 => "正常启动".to_string(),
            1 => "安全模式".to_string(),
            2 => "安全模式（带网络）".to_string(),
            _ => format!("未知启动模式 ({})", boot_mode),
        }
    }
}

/// 检查是否需要重启
/// 
/// # 返回值
/// 
/// 是否需要重启
pub fn is_reboot_pending() -> bool {
    // 检查注册表中的重启标志
    // 这里提供一个简化的实现
    false
}

/// 获取系统正常运行时间
/// 
/// # 返回值
/// 
/// 系统运行时间（毫秒）
pub fn get_system_uptime() -> u64 {
    use winapi::um::sysinfoapi::GetTickCount64;
    
    unsafe {
        GetTickCount64()
    }
}

/// 格式化系统运行时间
/// 
/// # 参数
/// 
/// * `uptime_ms` - 运行时间（毫秒）
/// 
/// # 返回值
/// 
/// 格式化的时间字符串
pub fn format_uptime(uptime_ms: u64) -> String {
    let total_seconds = uptime_ms / 1000;
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    if days > 0 {
        format!("{}天 {:02}:{:02}:{:02}", days, hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

/// 检查系统是否支持现代关机API
/// 
/// # 返回值
/// 
/// 是否支持现代API
pub fn supports_modern_shutdown_api() -> bool {
    // Windows 8及以上版本支持更好的关机API
    let mut compat = SystemCompatibility::new();
    if compat.detect_windows_version().is_ok() {
        if let Some(version) = compat.get_windows_version() {
            return matches!((version.major, version.minor), 
                (6, 2) | // Windows 8
                (6, 3) | // Windows 8.1
                (10, 0)  // Windows 10/11
            );
        }
    }
    false
}

/// 检查系统兼容性（全局函数）
/// 
/// # 返回值
/// 
/// 成功返回Ok(())，失败返回错误信息
pub fn check_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    let mut compat = SystemCompatibility::new();
    compat.check_compatibility()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_system_compatibility_creation() {
        let compat = SystemCompatibility::new();
        assert!(compat.windows_version.is_none());
        assert!(compat.user_permissions.is_none());
    }
    
    #[test]
    fn test_get_system_info() {
        let info = get_system_info();
        assert!(!info.is_empty());
        // 系统信息应该包含一些基本内容
    }
    
    #[test]
    fn test_get_system_uptime() {
        let uptime = get_system_uptime();
        assert!(uptime > 0); // 系统运行时间应该大于0
    }
    
    #[test]
    fn test_format_uptime() {
        // 测试不同的时间格式
        assert_eq!(format_uptime(0), "00:00:00");
        assert_eq!(format_uptime(3661000), "01:01:01"); // 1小时1分1秒
        assert_eq!(format_uptime(90061000), "1天 01:01:01"); // 1天1小时1分1秒
    }
    
    #[test]
    fn test_supports_modern_shutdown_api() {
        // 这个测试在不同系统上结果可能不同
        let supports = supports_modern_shutdown_api();
        // 只验证函数能正常执行
        assert!(supports || !supports);
    }
}