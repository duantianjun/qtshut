//! 系统托盘模块
//! 
//! 实现系统托盘图标、右键菜单和托盘交互功能

use log::{info, warn};
use tokio::sync::mpsc;
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent, 
    menu::{Menu, MenuItem, MenuEvent, PredefinedMenuItem},
    Icon
};

use crate::core::types::{UIEvent, CountdownStatus};

/// 托盘图标管理器
/// 
/// 负责创建和管理系统托盘图标及其菜单
pub struct TrayManager {
    /// 托盘图标
    tray_icon: Option<TrayIcon>,
    /// 托盘菜单
    tray_menu: Option<Menu>,
    /// UI事件发送器
    ui_event_sender: mpsc::UnboundedSender<UIEvent>,
    /// 当前倒计时状态
    current_status: CountdownStatus,
    /// 菜单项ID
    menu_items: TrayMenuItems,
}

/// 托盘菜单项ID
#[derive(Debug, Clone)]
struct TrayMenuItems {
    /// 显示/隐藏主窗口
    show_hide: String,
    /// 开始倒计时
    start_countdown: String,
    /// 取消倒计时
    cancel_countdown: String,
    /// 设置
    settings: String,
    /// 关于
    about: String,
    /// 退出
    quit: String,
}

impl Default for TrayMenuItems {
    fn default() -> Self {
        Self {
            show_hide: "show_hide".to_string(),
            start_countdown: "start_countdown".to_string(),
            cancel_countdown: "cancel_countdown".to_string(),
            settings: "settings".to_string(),
            about: "about".to_string(),
            quit: "quit".to_string(),
        }
    }
}

impl TrayManager {
    /// 创建新的托盘管理器
    /// 
    /// # 参数
    /// 
    /// * `ui_event_sender` - UI事件发送器
    pub fn new(ui_event_sender: mpsc::UnboundedSender<UIEvent>) -> Self {
        Self {
            tray_icon: None,
            tray_menu: None,
            ui_event_sender,
            current_status: CountdownStatus::Idle,
            menu_items: TrayMenuItems::default(),
        }
    }
    
    /// 初始化托盘图标
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("初始化系统托盘图标");
        
        // 创建托盘菜单
        let menu = self.create_tray_menu()?;
        
        // 加载托盘图标
        let icon = self.load_tray_icon()?;
        
        // 创建托盘图标
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("QtShut - 定时关机")
            .with_icon(icon)
            .build()?;
        
        self.tray_icon = Some(tray_icon);
        self.tray_menu = Some(menu);
        
        info!("系统托盘图标初始化成功");
        Ok(())
    }
    
    /// 创建托盘菜单
    /// 
    /// # 返回值
    /// 
    /// 成功返回菜单对象
    fn create_tray_menu(&self) -> Result<Menu, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        
        // 显示/隐藏主窗口
        let show_hide_item = MenuItem::new("显示主窗口", true, None);
        menu.append(&show_hide_item)?;
        
        // 分隔符
        menu.append(&PredefinedMenuItem::separator())?;
        
        // 开始倒计时
        let start_item = MenuItem::new("快速倒计时 (30分钟)", true, None);
        menu.append(&start_item)?;
        
        // 取消倒计时
        let cancel_item = MenuItem::new("取消倒计时", false, None); // 初始禁用
        menu.append(&cancel_item)?;
        
        // 分隔符
        menu.append(&PredefinedMenuItem::separator())?;
        
        // 设置
        let settings_item = MenuItem::new("设置", true, None);
        menu.append(&settings_item)?;
        
        // 关于
        let about_item = MenuItem::new("关于", true, None);
        menu.append(&about_item)?;
        
        // 分隔符
        menu.append(&PredefinedMenuItem::separator())?;
        
        // 退出
        let quit_item = MenuItem::new("退出", true, None);
        menu.append(&quit_item)?;
        
        Ok(menu)
    }
    
    /// 加载托盘图标
    /// 
    /// # 返回值
    /// 
    /// 成功返回图标对象
    fn load_tray_icon(&self) -> Result<Icon, Box<dyn std::error::Error>> {
        // 尝试从资源加载图标，如果失败则使用默认图标
        match self.load_icon_from_resource() {
            Ok(icon) => Ok(icon),
            Err(e) => {
                warn!("加载资源图标失败: {}, 使用默认图标", e);
                self.create_default_icon()
            }
        }
    }
    
    /// 从资源加载图标
    /// 
    /// # 返回值
    /// 
    /// 成功返回图标对象
    fn load_icon_from_resource(&self) -> Result<Icon, Box<dyn std::error::Error>> {
        // 尝试加载嵌入的图标资源
        // 这里可以使用include_bytes!宏嵌入图标文件
        
        // 示例：如果有icon.ico文件
        // let icon_data = include_bytes!("../../assets/icon.ico");
        // Icon::from_resource(icon_data, None)
        
        // 暂时返回错误，使用默认图标
        Err("未找到资源图标".into())
    }
    
    /// 创建默认图标
    /// 
    /// # 返回值
    /// 
    /// 成功返回默认图标对象
    fn create_default_icon(&self) -> Result<Icon, Box<dyn std::error::Error>> {
        // 创建一个简单的16x16像素图标
        let icon_data = self.generate_default_icon_data();
        
        Icon::from_rgba(icon_data, 16, 16)
            .map_err(|e| format!("创建默认图标失败: {}", e).into())
    }
    
    /// 生成默认图标数据
    /// 
    /// # 返回值
    /// 
    /// RGBA格式的图标数据
    fn generate_default_icon_data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(16 * 16 * 4);
        
        // 创建一个简单的16x16红色方块图标
        for y in 0..16 {
            for x in 0..16 {
                if x >= 2 && x <= 13 && y >= 2 && y <= 13 {
                    // 内部红色
                    data.extend_from_slice(&[200, 50, 50, 255]); // RGBA
                } else {
                    // 边框黑色
                    data.extend_from_slice(&[0, 0, 0, 255]); // RGBA
                }
            }
        }
        
        data
    }
    
    /// 处理托盘事件
    /// 
    /// # 参数
    /// 
    /// * `event` - 托盘图标事件
    pub fn handle_tray_event(&mut self, event: TrayIconEvent) {
        match event {
            TrayIconEvent::Click { button, button_state, .. } => {
                info!("托盘图标点击事件: {:?}, {:?}", button, button_state);
                
                // 左键单击显示/隐藏主窗口
                if button == tray_icon::MouseButton::Left {
                    let _ = self.ui_event_sender.send(UIEvent::ToggleMainWindow);
                }
            },
            // 注意: tray_icon crate 可能不支持 DoubleClick 事件
            // 如果需要双击功能，可以通过计时器实现
            _ => {}
        }
    }
    
    /// 处理菜单事件
    /// 
    /// # 参数
    /// 
    /// * `event` - 菜单事件
    pub fn handle_menu_event(&mut self, event: MenuEvent) {
        let menu_id = event.id.0;
        info!("托盘菜单点击: {}", menu_id);
        
        // 根据菜单ID处理不同的事件
        if menu_id == self.menu_items.show_hide {
            let _ = self.ui_event_sender.send(UIEvent::ToggleMainWindow);
        } else if menu_id == self.menu_items.start_countdown {
            // 快速开始30分钟倒计时
            let _ = self.ui_event_sender.send(UIEvent::QuickCountdown(30));
        } else if menu_id == self.menu_items.cancel_countdown {
            let _ = self.ui_event_sender.send(UIEvent::CancelCountdown);
        } else if menu_id == self.menu_items.settings {
            let _ = self.ui_event_sender.send(UIEvent::ShowSettings);
        } else if menu_id == self.menu_items.about {
            let _ = self.ui_event_sender.send(UIEvent::ShowAbout);
        } else if menu_id == self.menu_items.quit {
            let _ = self.ui_event_sender.send(UIEvent::Exit);
        }
    }
    
    /// 更新托盘图标状态
    /// 
    /// # 参数
    /// 
    /// * `status` - 新的倒计时状态
    pub fn update_status(&mut self, status: CountdownStatus) {
        self.current_status = status.clone();
        
        // 更新托盘图标提示文本
        if let Some(_tray_icon) = &self.tray_icon {
            let tooltip = self.generate_tooltip(&status);
            if let Err(e) = _tray_icon.set_tooltip(Some(&tooltip)) {
                warn!("更新托盘提示失败: {}", e);
            }
        }
        
        // 更新菜单项状态
        self.update_menu_items(&status);
    }
    
    /// 生成提示文本
    /// 
    /// # 参数
    /// 
    /// * `status` - 倒计时状态
    /// 
    /// # 返回值
    /// 
    /// 提示文本字符串
    fn generate_tooltip(&self, status: &CountdownStatus) -> String {
        match status {
            CountdownStatus::Idle => "QtShut - 定时关机 (空闲)".to_string(),
            CountdownStatus::Running { remaining } => {
                let time_str = self.format_duration(remaining);
                format!("QtShut - 剩余时间: {}", time_str)
            },
            CountdownStatus::Finished => "QtShut - 倒计时结束".to_string(),
            CountdownStatus::Cancelled => "QtShut - 任务已取消".to_string(),
            CountdownStatus::Error(msg) => format!("QtShut - 错误: {}", msg),
        }
    }
    
    /// 格式化时间间隔
    /// 
    /// # 参数
    /// 
    /// * `duration` - 时间间隔
    /// 
    /// # 返回值
    /// 
    /// 格式化的时间字符串
    fn format_duration(&self, duration: &chrono::Duration) -> String {
        let total_seconds = duration.num_seconds();
        
        if total_seconds <= 0 {
            return "00:00".to_string();
        }
        
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        
        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }
    
    /// 更新菜单项状态
    /// 
    /// # 参数
    /// 
    /// * `status` - 倒计时状态
    fn update_menu_items(&self, status: &CountdownStatus) {
        if let Some(_menu) = &self.tray_menu {
            // 根据状态启用/禁用相应的菜单项
            match status {
                CountdownStatus::Idle | CountdownStatus::Cancelled | CountdownStatus::Error(_) => {
                    // 启用开始倒计时，禁用取消倒计时
                    // 注意：tray-icon库的菜单项更新API可能有所不同
                    // 这里提供一个概念性的实现
                },
                CountdownStatus::Running { .. } => {
                    // 禁用开始倒计时，启用取消倒计时
                },
                CountdownStatus::Finished => {
                    // 禁用所有倒计时相关操作
                }
            }
        }
    }
    
    /// 显示托盘通知
    /// 
    /// # 参数
    /// 
    /// * `title` - 通知标题
    /// * `message` - 通知内容
    pub fn show_notification(&self, title: &str, message: &str) {
        info!("显示托盘通知: {} - {}", title, message);
        
        // 在Windows上可以使用系统通知
        // 这里提供一个简化的实现
        if let Some(_tray_icon) = &self.tray_icon {
            // 注意：具体的通知API取决于使用的库
            // 可能需要额外的通知库如notify-rust
            info!("通知: {} - {}", title, message);
        }
    }
    
    /// 销毁托盘图标
    pub fn destroy(&mut self) {
        info!("销毁系统托盘图标");
        
        if let Some(tray_icon) = self.tray_icon.take() {
            // 托盘图标会在drop时自动清理
            drop(tray_icon);
        }
        
        self.tray_menu = None;
    }
    
    /// 清理托盘资源
    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.destroy();
        Ok(())
    }
}

/// 托盘事件处理器
/// 
/// 用于处理托盘相关的事件
#[derive(Debug)]
pub struct TrayEventHandler {
    /// UI事件发送器
    ui_event_sender: mpsc::UnboundedSender<UIEvent>,
}

impl TrayEventHandler {
    /// 创建新的托盘事件处理器
    /// 
    /// # 参数
    /// 
    /// * `ui_event_sender` - UI事件发送器
    pub fn new(ui_event_sender: mpsc::UnboundedSender<UIEvent>) -> Self {
        Self { ui_event_sender }
    }
    
    /// 处理托盘图标事件
    /// 
    /// # 参数
    /// 
    /// * `event` - 托盘图标事件
    pub fn handle_tray_icon_event(&self, event: TrayIconEvent) {
        match event {
            TrayIconEvent::Click { button, button_state: _, .. } => {
                if button == tray_icon::MouseButton::Left {
                    let _ = self.ui_event_sender.send(UIEvent::ToggleMainWindow);
                }
            },

            _ => {}
        }
    }
    
    /// 处理菜单事件
    /// 
    /// # 参数
    /// 
    /// * `event` - 菜单事件
    pub fn handle_menu_event(&self, event: MenuEvent) {
        // 根据菜单项ID发送相应的UI事件
        let menu_id = event.id.0;
        
        // 这里需要根据实际的菜单项ID进行匹配
        // 具体实现取决于菜单的创建方式
        info!("处理菜单事件: {}", menu_id);
    }
}

impl std::fmt::Debug for TrayManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayManager")
            .field("current_status", &self.current_status)
            .field("menu_items", &self.menu_items)
            .field("tray_icon", &"<TrayIcon>")
            .field("tray_menu", &"<Menu>")
            .field("ui_event_sender", &"<Sender>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    
    #[test]
    fn test_tray_manager_creation() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let manager = TrayManager::new(sender);
        
        assert!(manager.tray_icon.is_none());
        assert!(manager.tray_menu.is_none());
        assert!(matches!(manager.current_status, CountdownStatus::Idle));
    }
    
    #[test]
    fn test_default_icon_generation() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let manager = TrayManager::new(sender);
        
        let icon_data = manager.generate_default_icon_data();
        assert_eq!(icon_data.len(), 16 * 16 * 4); // 16x16 RGBA
    }
    
    #[test]
    fn test_tooltip_generation() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let manager = TrayManager::new(sender);
        
        let idle_tooltip = manager.generate_tooltip(&CountdownStatus::Idle);
        assert!(idle_tooltip.contains("空闲"));
        
        let running_tooltip = manager.generate_tooltip(&CountdownStatus::Running {
            remaining: chrono::Duration::minutes(30)
        });
        assert!(running_tooltip.contains("剩余时间"));
    }
    
    #[test]
    fn test_duration_formatting() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let manager = TrayManager::new(sender);
        
        let duration = chrono::Duration::seconds(3661); // 1小时1分1秒
        assert_eq!(manager.format_duration(&duration), "01:01:01");
        
        let duration = chrono::Duration::seconds(61); // 1分1秒
        assert_eq!(manager.format_duration(&duration), "01:01");
    }
}