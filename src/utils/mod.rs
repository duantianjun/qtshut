//! 工具模块
//! 
//! 包含各种实用工具函数和辅助功能

pub mod system;
pub mod config;
pub mod logger;
pub mod notification;

// 为了兼容性，将system模块也作为system_compat导出
pub mod system_compat {
    
}

// 重新导出常用功能
