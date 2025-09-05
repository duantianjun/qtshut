//! 用户界面模块
//! 
//! 包含GUI界面、托盘图标和用户交互相关的所有组件

pub mod manager;
pub mod components;
pub mod tray;
pub mod theme;

// 重新导出主要组件
pub use manager::UIManager;
