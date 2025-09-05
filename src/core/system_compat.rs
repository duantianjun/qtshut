//! 系统兼容性适配模块
//! 
//! 提供Windows版本检测、权限检查和系统兼容性功能

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use winapi::um::sysinfoapi::GetVersionExW;
use winapi::um::winnt::OSVERSIONINFOEXW;
use winapi::um::winnt::VER_NT_WORKSTATION;
use winapi::shared::minwindef::{DWORD, FALSE};
use anyhow::Result;
use log::info;

/// Windows版本信息
#[derive(Debug, Clone)]
pub struct WindowsVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub service_pack: String,
    pub product_type: WindowsProductType,
    pub version_name: String,
}

/// Windows产品类型
#[derive(Debug, Clone, PartialEq)]
pub enum WindowsProductType {
    Workstation,
    Server,
    Unknown,
}

/// 系统兼容性检查器
#[derive(Debug)]
pub struct SystemCompatibility {
    windows_version: Option<WindowsVersion>,
}

impl SystemCompatibility {
    /// 创建新的系统兼容性检查器
    pub fn new() -> Self {
        Self {
            windows_version: None,
        }
    }

    /// 初始化系统信息检测
    pub async fn initialize(&mut self) -> Result<()> {
        info!("正在初始化系统兼容性检查器...");
        
        // 检测Windows版本
        self.windows_version = Some(self.detect_windows_version()?);
        
        if let Some(ref version) = self.windows_version {
            info!("检测到Windows版本: {} (Build {})", version.version_name, version.build);
        }
        
        Ok(())
    }

    /// 检测Windows版本
    fn detect_windows_version(&self) -> Result<WindowsVersion> {
        unsafe {
            let mut version_info: OSVERSIONINFOEXW = std::mem::zeroed();
            version_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as DWORD;
            
            if GetVersionExW(&mut version_info as *mut _ as *mut _) == FALSE {
                return Err(anyhow::anyhow!("无法获取Windows版本信息"));
            }
            
            let major = version_info.dwMajorVersion;
            let minor = version_info.dwMinorVersion;
            let build = version_info.dwBuildNumber;
            
            // 转换服务包信息
            let service_pack = {
                let sp_slice = std::slice::from_raw_parts(
                    version_info.szCSDVersion.as_ptr(),
                    version_info.szCSDVersion.len(),
                );
                let end = sp_slice.iter().position(|&c| c == 0).unwrap_or(sp_slice.len());
                OsString::from_wide(&sp_slice[..end])
                    .to_string_lossy()
                    .to_string()
            };
            
            // 确定产品类型
            let product_type = if version_info.wProductType == VER_NT_WORKSTATION {
                WindowsProductType::Workstation
            } else {
                WindowsProductType::Server
            };
            
            // 确定版本名称
            let version_name = self.get_windows_version_name(major, minor, build, &product_type);
            
            Ok(WindowsVersion {
                major,
                minor,
                build,
                service_pack,
                product_type,
                version_name,
            })
        }
    }

    /// 根据版本号获取Windows版本名称
    fn get_windows_version_name(
        &self,
        major: u32,
        minor: u32,
        build: u32,
        product_type: &WindowsProductType,
    ) -> String {
        match (major, minor) {
            (10, 0) => {
                if build >= 22000 {
                    "Windows 11".to_string()
                } else {
                    match product_type {
                        WindowsProductType::Workstation => "Windows 10".to_string(),
                        WindowsProductType::Server => "Windows Server 2016/2019/2022".to_string(),
                        _ => "Windows 10/Server".to_string(),
                    }
                }
            }
            (6, 3) => {
                match product_type {
                    WindowsProductType::Workstation => "Windows 8.1".to_string(),
                    WindowsProductType::Server => "Windows Server 2012 R2".to_string(),
                    _ => "Windows 8.1/Server 2012 R2".to_string(),
                }
            }
            (6, 2) => {
                match product_type {
                    WindowsProductType::Workstation => "Windows 8".to_string(),
                    WindowsProductType::Server => "Windows Server 2012".to_string(),
                    _ => "Windows 8/Server 2012".to_string(),
                }
            }
            (6, 1) => {
                match product_type {
                    WindowsProductType::Workstation => "Windows 7".to_string(),
                    WindowsProductType::Server => "Windows Server 2008 R2".to_string(),
                    _ => "Windows 7/Server 2008 R2".to_string(),
                }
            }
            (6, 0) => {
                match product_type {
                    WindowsProductType::Workstation => "Windows Vista".to_string(),
                    WindowsProductType::Server => "Windows Server 2008".to_string(),
                    _ => "Windows Vista/Server 2008".to_string(),
                }
            }
            _ => format!("Windows {}.{}", major, minor),
        }
    }

    /// 检查系统是否支持定时关机功能
    pub fn is_shutdown_supported(&self) -> bool {
        if let Some(ref version) = self.windows_version {
            // Windows Vista (6.0) 及以上版本都支持
            version.major >= 6
        } else {
            false
        }
    }

    /// 检查是否需要管理员权限
    pub fn requires_admin_privileges(&self) -> bool {
        if let Some(ref version) = self.windows_version {
            // Windows Vista (6.0) 及以上版本需要UAC权限
            version.major >= 6
        } else {
            true // 未知版本，保守起见要求管理员权限
        }
    }

    /// 获取Windows版本信息
    pub fn get_windows_version(&self) -> Option<&WindowsVersion> {
        self.windows_version.as_ref()
    }

    /// 检查当前进程是否具有管理员权限
    pub fn has_admin_privileges(&self) -> bool {
        use winapi::um::processthreadsapi::GetCurrentProcess;
        use winapi::um::securitybaseapi::GetTokenInformation;
        use winapi::um::winnt::{
            TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY, HANDLE,
        };
        use winapi::um::handleapi::CloseHandle;
        
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            let process = GetCurrentProcess();
            
            // 打开进程令牌
            let result = winapi::um::processthreadsapi::OpenProcessToken(
                process,
                TOKEN_QUERY,
                &mut token,
            );
            
            if result == FALSE || token.is_null() {
                return false;
            }
            
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut return_length: DWORD = 0;
            
            // 查询令牌提升信息
            let result = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                std::mem::size_of::<TOKEN_ELEVATION>() as DWORD,
                &mut return_length,
            );
            
            CloseHandle(token);
            
            if result == FALSE {
                return false;
            }
            
            elevation.TokenIsElevated != 0
        }
    }

    /// 生成系统兼容性报告
    pub fn generate_compatibility_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== QtShut 系统兼容性报告 ===\n\n");
        
        if let Some(ref version) = self.windows_version {
            report.push_str(&format!("操作系统: {}\n", version.version_name));
            report.push_str(&format!("版本号: {}.{}.{}\n", version.major, version.minor, version.build));
            if !version.service_pack.is_empty() {
                report.push_str(&format!("服务包: {}\n", version.service_pack));
            }
            report.push_str(&format!("产品类型: {:?}\n\n", version.product_type));
        } else {
            report.push_str("操作系统: 未知\n\n");
        }
        
        report.push_str(&format!("定时关机支持: {}\n", 
            if self.is_shutdown_supported() { "是" } else { "否" }));
        report.push_str(&format!("需要管理员权限: {}\n", 
            if self.requires_admin_privileges() { "是" } else { "否" }));
        report.push_str(&format!("当前具有管理员权限: {}\n", 
            if self.has_admin_privileges() { "是" } else { "否" }));
        
        if self.requires_admin_privileges() && !self.has_admin_privileges() {
            report.push_str("\n⚠️  警告: 当前程序没有管理员权限，可能无法执行关机操作。\n");
            report.push_str("建议以管理员身份运行程序。\n");
        }
        
        if !self.is_shutdown_supported() {
            report.push_str("\n❌ 错误: 当前系统不支持定时关机功能。\n");
        }
        
        report
    }
}

impl Default for SystemCompatibility {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_system_compatibility_initialization() {
        let mut compat = SystemCompatibility::new();
        let result = compat.initialize().await;
        assert!(result.is_ok());
        assert!(compat.get_windows_version().is_some());
    }
    
    #[test]
    fn test_windows_version_name_detection() {
        let compat = SystemCompatibility::new();
        
        // 测试Windows 10
        let name = compat.get_windows_version_name(10, 0, 19041, &WindowsProductType::Workstation);
        assert_eq!(name, "Windows 10");
        
        // 测试Windows 11
        let name = compat.get_windows_version_name(10, 0, 22000, &WindowsProductType::Workstation);
        assert_eq!(name, "Windows 11");
        
        // 测试Windows 7
        let name = compat.get_windows_version_name(6, 1, 7601, &WindowsProductType::Workstation);
        assert_eq!(name, "Windows 7");
    }
    
    #[test]
    fn test_shutdown_support_check() {
        let mut compat = SystemCompatibility::new();
        
        // 模拟Windows 10
        compat.windows_version = Some(WindowsVersion {
            major: 10,
            minor: 0,
            build: 19041,
            service_pack: String::new(),
            product_type: WindowsProductType::Workstation,
            version_name: "Windows 10".to_string(),
        });
        
        assert!(compat.is_shutdown_supported());
        assert!(compat.requires_admin_privileges());
    }
    
    #[test]
    fn test_compatibility_report_generation() {
        let mut compat = SystemCompatibility::new();
        
        // 模拟Windows 10
        compat.windows_version = Some(WindowsVersion {
            major: 10,
            minor: 0,
            build: 19041,
            service_pack: String::new(),
            product_type: WindowsProductType::Workstation,
            version_name: "Windows 10".to_string(),
        });
        
        let report = compat.generate_compatibility_report();
        assert!(report.contains("Windows 10"));
        assert!(report.contains("定时关机支持: 是"));
    }
}