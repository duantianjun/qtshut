//! 通知管理模块
//! 
//! 负责系统通知的显示和管理

use log::info;

/// 通知类型
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationType {
    /// 信息通知
    Info,
    /// 警告通知
    Warning,
    /// 错误通知
    Error,
    /// 成功通知
    Success,
    /// 倒计时通知
    Countdown,
}

/// 通知优先级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationPriority {
    /// 低优先级
    Low,
    /// 普通优先级
    Normal,
    /// 高优先级
    High,
    /// 紧急优先级
    Critical,
}

/// 通知消息
#[derive(Debug, Clone)]
pub struct NotificationMessage {
    /// 通知ID
    pub id: String,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 通知类型
    pub notification_type: NotificationType,
    /// 优先级
    pub priority: NotificationPriority,
    /// 显示持续时间（毫秒）
    pub duration_ms: Option<u64>,
    /// 是否可关闭
    pub dismissible: bool,
    /// 是否播放声音
    pub play_sound: bool,
    /// 创建时间
    pub created_at: std::time::Instant,
}

impl NotificationMessage {
    /// 创建新的通知消息
    /// 
    /// # 参数
    /// 
    /// * `title` - 标题
    /// * `content` - 内容
    /// * `notification_type` - 通知类型
    /// 
    /// # 返回值
    /// 
    /// 通知消息
    pub fn new(
        title: impl Into<String>,
        content: impl Into<String>,
        notification_type: NotificationType,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        
        Self {
            id,
            title: title.into(),
            content: content.into(),
            notification_type,
            priority: NotificationPriority::Normal,
            duration_ms: Some(5000), // 默认5秒
            dismissible: true,
            play_sound: false,
            created_at: std::time::Instant::now(),
        }
    }
    
    /// 设置优先级
    /// 
    /// # 参数
    /// 
    /// * `priority` - 优先级
    /// 
    /// # 返回值
    /// 
    /// 自身的可变引用，支持链式调用
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// 设置显示持续时间
    /// 
    /// # 参数
    /// 
    /// * `duration_ms` - 持续时间（毫秒），None表示不自动消失
    /// 
    /// # 返回值
    /// 
    /// 自身的可变引用，支持链式调用
    pub fn with_duration(mut self, duration_ms: Option<u64>) -> Self {
        self.duration_ms = duration_ms;
        self
    }
    
    /// 设置是否可关闭
    /// 
    /// # 参数
    /// 
    /// * `dismissible` - 是否可关闭
    /// 
    /// # 返回值
    /// 
    /// 自身的可变引用，支持链式调用
    pub fn with_dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }
    
    /// 设置是否播放声音
    /// 
    /// # 参数
    /// 
    /// * `play_sound` - 是否播放声音
    /// 
    /// # 返回值
    /// 
    /// 自身的可变引用，支持链式调用
    pub fn with_sound(mut self, play_sound: bool) -> Self {
        self.play_sound = play_sound;
        self
    }
    
    /// 检查通知是否已过期
    /// 
    /// # 返回值
    /// 
    /// 是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(duration_ms) = self.duration_ms {
            let elapsed = self.created_at.elapsed();
            elapsed.as_millis() > duration_ms as u128
        } else {
            false
        }
    }
    
    /// 获取剩余显示时间
    /// 
    /// # 返回值
    /// 
    /// 剩余时间（毫秒），None表示永久显示
    pub fn remaining_time_ms(&self) -> Option<u64> {
        if let Some(duration_ms) = self.duration_ms {
            let elapsed_ms = self.created_at.elapsed().as_millis() as u64;
            if elapsed_ms < duration_ms {
                Some(duration_ms - elapsed_ms)
            } else {
                Some(0)
            }
        } else {
            None
        }
    }
}

/// 通知管理器
/// 
/// 负责通知的显示、管理和清理
#[derive(Debug)]
pub struct NotificationManager {
    /// 当前活跃的通知
    active_notifications: Vec<NotificationMessage>,
    /// 最大通知数量
    max_notifications: usize,
    /// 是否启用通知
    enabled: bool,
    /// 是否启用声音
    sound_enabled: bool,
    /// 通知历史
    notification_history: Vec<NotificationMessage>,
    /// 最大历史记录数量
    max_history: usize,
}

impl NotificationManager {
    /// 创建新的通知管理器
    /// 
    /// # 参数
    /// 
    /// * `max_notifications` - 最大同时显示的通知数量
    /// * `max_history` - 最大历史记录数量
    /// 
    /// # 返回值
    /// 
    /// 通知管理器
    pub fn new(max_notifications: usize, max_history: usize) -> Self {
        Self {
            active_notifications: Vec::new(),
            max_notifications,
            enabled: true,
            sound_enabled: true,
            notification_history: Vec::new(),
            max_history,
        }
    }
    
    /// 显示通知
    /// 
    /// # 参数
    /// 
    /// * `notification` - 通知消息
    /// 
    /// # 返回值
    /// 
    /// 成功返回通知ID，失败返回错误信息
    pub async fn show_notification(
        &mut self,
        notification: NotificationMessage,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if !self.enabled {
            info!("通知已禁用，跳过显示: {}", notification.title);
            return Ok(notification.id);
        }
        
        info!("显示通知: {} - {}", notification.title, notification.content);
        
        // 播放声音（如果启用）
        if notification.play_sound && self.sound_enabled {
            self.play_notification_sound(&notification.notification_type).await;
        }
        
        // 检查是否需要移除旧通知
        if self.active_notifications.len() >= self.max_notifications {
            // 移除最旧的低优先级通知
            self.remove_oldest_low_priority_notification();
        }
        
        let notification_id = notification.id.clone();
        
        // 添加到活跃通知列表
        self.active_notifications.push(notification.clone());
        
        // 按优先级排序
        self.active_notifications.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // 添加到历史记录
        self.add_to_history(notification);
        
        // 显示系统通知（Windows）
        self.show_system_notification(&notification_id).await?;
        
        Ok(notification_id)
    }
    
    /// 显示系统通知
    /// 
    /// # 参数
    /// 
    /// * `notification_id` - 通知ID
    /// 
    /// # 返回值
    /// 
    /// 成功返回Ok(())，失败返回错误信息
    async fn show_system_notification(
        &self,
        notification_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(notification) = self.get_notification_by_id(notification_id) {
            // 在Windows上显示系统通知
            // 这里使用简单的实现，实际项目中可能需要使用专门的通知库
            #[cfg(target_os = "windows")]
            {
                // 使用Windows API显示通知
                // 这里是一个简化的实现
                info!("Windows系统通知: {} - {}", notification.title, notification.content);
            }
            
            #[cfg(not(target_os = "windows"))]
            {
                info!("系统通知: {} - {}", notification.title, notification.content);
            }
        }
        
        Ok(())
    }
    
    /// 播放通知声音
    /// 
    /// # 参数
    /// 
    /// * `notification_type` - 通知类型
    async fn play_notification_sound(&self, notification_type: &NotificationType) {
        // 根据通知类型播放不同的声音
        match notification_type {
            NotificationType::Info => {
                info!("播放信息通知声音");
                // 播放信息声音
            },
            NotificationType::Warning => {
                info!("播放警告通知声音");
                // 播放警告声音
            },
            NotificationType::Error => {
                info!("播放错误通知声音");
                // 播放错误声音
            },
            NotificationType::Success => {
                info!("播放成功通知声音");
                // 播放成功声音
            },
            NotificationType::Countdown => {
                info!("播放倒计时通知声音");
                // 播放倒计时声音
            },
        }
        
        // 在Windows上可以使用MessageBeep API
        #[cfg(target_os = "windows")]
        {
            // 使用Windows API播放系统声音
            // 这里是一个简化的实现
        }
    }
    
    /// 关闭通知
    /// 
    /// # 参数
    /// 
    /// * `notification_id` - 通知ID
    /// 
    /// # 返回值
    /// 
    /// 是否成功关闭
    pub fn dismiss_notification(&mut self, notification_id: &str) -> bool {
        if let Some(pos) = self.active_notifications.iter().position(|n| n.id == notification_id) {
            let notification = self.active_notifications.remove(pos);
            info!("关闭通知: {}", notification.title);
            true
        } else {
            false
        }
    }
    
    /// 关闭所有通知
    pub fn dismiss_all_notifications(&mut self) {
        let count = self.active_notifications.len();
        self.active_notifications.clear();
        info!("关闭所有通知，共 {} 个", count);
    }
    
    /// 清理过期通知
    /// 
    /// # 返回值
    /// 
    /// 清理的通知数量
    pub fn cleanup_expired_notifications(&mut self) -> usize {
        let initial_count = self.active_notifications.len();
        
        self.active_notifications.retain(|notification| {
            if notification.is_expired() {
                info!("清理过期通知: {}", notification.title);
                false
            } else {
                true
            }
        });
        
        let cleaned_count = initial_count - self.active_notifications.len();
        
        if cleaned_count > 0 {
            info!("清理了 {} 个过期通知", cleaned_count);
        }
        
        cleaned_count
    }
    
    /// 获取活跃通知列表
    pub fn get_active_notifications(&self) -> &[NotificationMessage] {
        &self.active_notifications
    }
    
    /// 获取通知历史
    pub fn get_notification_history(&self) -> &[NotificationMessage] {
        &self.notification_history
    }
    
    /// 根据ID获取通知
    /// 
    /// # 参数
    /// 
    /// * `notification_id` - 通知ID
    /// 
    /// # 返回值
    /// 
    /// 通知消息的引用
    pub fn get_notification_by_id(&self, notification_id: &str) -> Option<&NotificationMessage> {
        self.active_notifications.iter().find(|n| n.id == notification_id)
    }
    
    /// 启用或禁用通知
    /// 
    /// # 参数
    /// 
    /// * `enabled` - 是否启用
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        info!("通知系统 {}", if enabled { "已启用" } else { "已禁用" });
    }
    
    /// 启用或禁用声音
    /// 
    /// # 参数
    /// 
    /// * `enabled` - 是否启用声音
    pub fn set_sound_enabled(&mut self, enabled: bool) {
        self.sound_enabled = enabled;
        info!("通知声音 {}", if enabled { "已启用" } else { "已禁用" });
    }
    
    /// 获取通知统计信息
    /// 
    /// # 返回值
    /// 
    /// 通知统计信息
    pub fn get_stats(&self) -> NotificationStats {
        let mut stats = NotificationStats::default();
        
        stats.active_count = self.active_notifications.len();
        stats.history_count = self.notification_history.len();
        stats.enabled = self.enabled;
        stats.sound_enabled = self.sound_enabled;
        
        // 按类型统计
        for notification in &self.active_notifications {
            match notification.notification_type {
                NotificationType::Info => stats.info_count += 1,
                NotificationType::Warning => stats.warning_count += 1,
                NotificationType::Error => stats.error_count += 1,
                NotificationType::Success => stats.success_count += 1,
                NotificationType::Countdown => stats.countdown_count += 1,
            }
        }
        
        stats
    }
    
    /// 移除最旧的低优先级通知
    fn remove_oldest_low_priority_notification(&mut self) {
        // 查找最旧的低优先级通知
        if let Some(pos) = self.active_notifications.iter().position(|n| {
            matches!(n.priority, NotificationPriority::Low | NotificationPriority::Normal)
        }) {
            let notification = self.active_notifications.remove(pos);
            info!("移除旧通知为新通知腾出空间: {}", notification.title);
        } else if !self.active_notifications.is_empty() {
            // 如果没有低优先级通知，移除最旧的通知
            let notification = self.active_notifications.remove(0);
            info!("移除最旧通知为新通知腾出空间: {}", notification.title);
        }
    }
    
    /// 添加到历史记录
    /// 
    /// # 参数
    /// 
    /// * `notification` - 通知消息
    fn add_to_history(&mut self, notification: NotificationMessage) {
        self.notification_history.push(notification);
        
        // 限制历史记录数量
        if self.notification_history.len() > self.max_history {
            self.notification_history.remove(0);
        }
    }
}

/// 通知统计信息
#[derive(Debug, Default, Clone)]
pub struct NotificationStats {
    /// 活跃通知数量
    pub active_count: usize,
    /// 历史通知数量
    pub history_count: usize,
    /// 是否启用通知
    pub enabled: bool,
    /// 是否启用声音
    pub sound_enabled: bool,
    /// 信息通知数量
    pub info_count: usize,
    /// 警告通知数量
    pub warning_count: usize,
    /// 错误通知数量
    pub error_count: usize,
    /// 成功通知数量
    pub success_count: usize,
    /// 倒计时通知数量
    pub countdown_count: usize,
}

/// 预定义的通知创建函数
pub struct NotificationBuilder;

impl NotificationBuilder {
    /// 创建信息通知
    /// 
    /// # 参数
    /// 
    /// * `title` - 标题
    /// * `content` - 内容
    /// 
    /// # 返回值
    /// 
    /// 通知消息
    pub fn info(title: impl Into<String>, content: impl Into<String>) -> NotificationMessage {
        NotificationMessage::new(title, content, NotificationType::Info)
    }
    
    /// 创建警告通知
    /// 
    /// # 参数
    /// 
    /// * `title` - 标题
    /// * `content` - 内容
    /// 
    /// # 返回值
    /// 
    /// 通知消息
    pub fn warning(title: impl Into<String>, content: impl Into<String>) -> NotificationMessage {
        NotificationMessage::new(title, content, NotificationType::Warning)
            .with_priority(NotificationPriority::High)
            .with_sound(true)
    }
    
    /// 创建错误通知
    /// 
    /// # 参数
    /// 
    /// * `title` - 标题
    /// * `content` - 内容
    /// 
    /// # 返回值
    /// 
    /// 通知消息
    pub fn error(title: impl Into<String>, content: impl Into<String>) -> NotificationMessage {
        NotificationMessage::new(title, content, NotificationType::Error)
            .with_priority(NotificationPriority::Critical)
            .with_duration(None) // 错误通知不自动消失
            .with_sound(true)
    }
    
    /// 创建成功通知
    /// 
    /// # 参数
    /// 
    /// * `title` - 标题
    /// * `content` - 内容
    /// 
    /// # 返回值
    /// 
    /// 通知消息
    pub fn success(title: impl Into<String>, content: impl Into<String>) -> NotificationMessage {
        NotificationMessage::new(title, content, NotificationType::Success)
            .with_duration(Some(3000)) // 成功通知3秒后消失
    }
    
    /// 创建倒计时通知
    /// 
    /// # 参数
    /// 
    /// * `title` - 标题
    /// * `content` - 内容
    /// 
    /// # 返回值
    /// 
    /// 通知消息
    pub fn countdown(title: impl Into<String>, content: impl Into<String>) -> NotificationMessage {
        NotificationMessage::new(title, content, NotificationType::Countdown)
            .with_priority(NotificationPriority::High)
            .with_duration(Some(10000)) // 倒计时通知10秒后消失
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[test]
    fn test_notification_message_creation() {
        let notification = NotificationMessage::new(
            "测试标题",
            "测试内容",
            NotificationType::Info,
        );
        
        assert_eq!(notification.title, "测试标题");
        assert_eq!(notification.content, "测试内容");
        assert_eq!(notification.notification_type, NotificationType::Info);
        assert_eq!(notification.priority, NotificationPriority::Normal);
        assert!(notification.dismissible);
        assert!(!notification.play_sound);
    }
    
    #[test]
    fn test_notification_builder_methods() {
        let notification = NotificationMessage::new("标题", "内容", NotificationType::Info)
            .with_priority(NotificationPriority::High)
            .with_duration(Some(3000))
            .with_dismissible(false)
            .with_sound(true);
        
        assert_eq!(notification.priority, NotificationPriority::High);
        assert_eq!(notification.duration_ms, Some(3000));
        assert!(!notification.dismissible);
        assert!(notification.play_sound);
    }
    
    #[tokio::test]
    async fn test_notification_expiration() {
        let mut notification = NotificationMessage::new(
            "测试",
            "内容",
            NotificationType::Info,
        ).with_duration(Some(100)); // 100ms后过期
        
        assert!(!notification.is_expired());
        
        sleep(Duration::from_millis(150)).await;
        
        assert!(notification.is_expired());
    }
    
    #[tokio::test]
    async fn test_notification_manager() {
        let mut manager = NotificationManager::new(5, 10);
        
        let notification = NotificationBuilder::info("测试", "内容");
        let id = manager.show_notification(notification).await.unwrap();
        
        assert_eq!(manager.get_active_notifications().len(), 1);
        assert!(manager.dismiss_notification(&id));
        assert_eq!(manager.get_active_notifications().len(), 0);
    }
    
    #[test]
    fn test_notification_builder() {
        let info = NotificationBuilder::info("信息", "内容");
        assert_eq!(info.notification_type, NotificationType::Info);
        
        let warning = NotificationBuilder::warning("警告", "内容");
        assert_eq!(warning.notification_type, NotificationType::Warning);
        assert_eq!(warning.priority, NotificationPriority::High);
        
        let error = NotificationBuilder::error("错误", "内容");
        assert_eq!(error.notification_type, NotificationType::Error);
        assert_eq!(error.priority, NotificationPriority::Critical);
        assert_eq!(error.duration_ms, None);
    }
    
    #[test]
    fn test_notification_stats() {
        let mut manager = NotificationManager::new(5, 10);
        let stats = manager.get_stats();
        
        assert_eq!(stats.active_count, 0);
        assert_eq!(stats.history_count, 0);
        assert!(stats.enabled);
        assert!(stats.sound_enabled);
    }
}