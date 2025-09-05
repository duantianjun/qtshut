//! 核心数据类型定义
//! 
//! 定义应用程序中使用的所有核心数据结构和枚举

use chrono::{DateTime, Local, NaiveTime, Duration};
use serde::{Deserialize, Serialize};
use std::fmt;

/// 任务类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    /// 单次关机任务
    Once,
    /// 每日重复关机任务
    Daily,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::Once => write!(f, "单次关机"),
            TaskType::Daily => write!(f, "每日关机"),
        }
    }
}

/// 时间输入类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum TimeInput {
    /// 相对时间间隔（如"30分钟"）
    Duration(Duration),
    /// 绝对时间点（如"今晚22:00"）
    AbsoluteTime(DateTime<Local>),
    /// 每日重复时间（如"22:00"）
    DailyTime(NaiveTime),
}

impl Default for TimeInput {
    fn default() -> Self {
        // 默认为30分钟的相对时间间隔
        TimeInput::Duration(Duration::minutes(30))
    }
}

/// 倒计时状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum CountdownStatus {
    /// 空闲状态，未设置任务
    Idle,
    /// 运行中，包含剩余时间
    Running { remaining: Duration },
    /// 已完成，等待执行关机
    Finished,
    /// 已取消
    Cancelled,
    /// 错误状态
    Error(String),
}

impl fmt::Display for CountdownStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CountdownStatus::Idle => write!(f, "未设置定时任务"),
            CountdownStatus::Running { remaining } => {
                let total_seconds = remaining.num_seconds();
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;
                
                if hours > 0 {
                    write!(f, "剩余{}小时{}分钟{}秒", hours, minutes, seconds)
                } else if minutes > 0 {
                    write!(f, "剩余{}分钟{}秒", minutes, seconds)
                } else {
                    write!(f, "剩余{}秒", seconds)
                }
            },
            CountdownStatus::Finished => write!(f, "倒计时结束，准备关机"),
            CountdownStatus::Cancelled => write!(f, "任务已取消"),
            CountdownStatus::Error(msg) => write!(f, "错误: {}", msg),
        }
    }
}

/// 任务数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskData {
    /// 任务类型
    pub task_type: TaskType,
    /// 目标时间（单次任务使用）
    pub target_time: Option<DateTime<Local>>,
    /// 每日时间（每日任务使用）
    pub daily_time: Option<NaiveTime>,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: DateTime<Local>,
}

/// 倒计时更新消息
#[derive(Debug, Clone)]
pub enum CountdownUpdate {
    /// 倒计时进度更新
    Progress { remaining: Duration, progress: f64 },
    /// 倒计时完成
    Finished,
    /// 倒计时取消
    Cancelled,
    /// 倒计时暂停
    Paused,
    /// 倒计时恢复
    Resumed,
    /// 任务完成
    TaskCompleted { task_info: TaskData },
    /// 倒计时错误
    Error(String),
}

/// UI事件枚举
#[derive(Debug, Clone)]
pub enum UIEvent {
    /// 开始倒计时
    StartCountdown(TimeInput, TaskType),
    /// 取消倒计时
    CancelCountdown,
    /// 最小化到托盘
    MinimizeToTray,
    /// 从托盘恢复
    RestoreFromTray,
    /// 显示主窗口
    ShowMainWindow,
    /// 切换主窗口显示状态
    ToggleMainWindow,
    /// 快速倒计时
    QuickCountdown(u32),
    /// 显示设置
    ShowSettings,
    /// 显示关于
    ShowAbout,
    /// 退出应用
    Exit,
}

/// 关机方法枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShutdownMethod {
    /// 使用shutdown.exe命令
    Command,
    /// 使用Windows API
    WinAPI,
}

/// Windows版本信息
#[derive(Debug, Clone)]
pub struct WindowsVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
}

impl WindowsVersion {
    /// Windows 11
    pub const Windows11: Self = Self { major: 10, minor: 0, build: 22000 };
    
    /// Windows 10
    pub const Windows10: Self = Self { major: 10, minor: 0, build: 10240 };
    
    /// Windows 8.1
    pub const Windows81: Self = Self { major: 6, minor: 3, build: 9600 };
    
    /// Windows 8
    pub const Windows8: Self = Self { major: 6, minor: 2, build: 9200 };
    
    /// Windows 7
    pub const Windows7: Self = Self { major: 6, minor: 1, build: 7601 };
    
    /// Windows Vista
    pub const WindowsVista: Self = Self { major: 6, minor: 0, build: 6002 };
    
    /// Windows XP
    pub const WindowsXP: Self = Self { major: 5, minor: 1, build: 2600 };
    
    /// 未知版本
    pub const Unknown: Self = Self { major: 0, minor: 0, build: 0 };
}

impl fmt::Display for WindowsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.build)
    }
}

/// 用户权限信息
#[derive(Debug, Clone)]
pub struct UserPermissions {
    pub can_shutdown: bool,
    pub is_admin: bool,
}