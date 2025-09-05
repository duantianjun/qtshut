//! 倒计时管理器模块
//! 
//! 负责管理倒计时状态，提供实时更新和任务调度功能

use anyhow::{Result, anyhow};
use chrono::{DateTime, Local, Duration, TimeZone};
use log::{info, error, debug};
use tokio::sync::{mpsc, broadcast, RwLock, Notify};
use tokio::time::{interval, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use uuid::Uuid;

use crate::core::types::{CountdownStatus, CountdownUpdate, TaskData, TaskType};
use crate::core::time_parser::TimeParser;

/// 倒计时管理器
#[derive(Debug)]
pub struct CountdownManager {
    /// 唯一标识符
    id: Uuid,
    /// 当前倒计时状态
    status: Arc<RwLock<CountdownStatus>>,
    /// 当前任务数据
    current_task: Arc<RwLock<Option<TaskData>>>,
    /// 更新通知发送器
    update_sender: broadcast::Sender<CountdownUpdate>,
    /// 取消令牌发送器
    cancel_sender: Arc<RwLock<Option<mpsc::UnboundedSender<()>>>>,
    /// 暂停/恢复通知
    pause_notify: Arc<Notify>,
    /// 是否暂停
    is_paused: Arc<AtomicBool>,
    /// 开始时间戳
    start_timestamp: Arc<AtomicU64>,
    /// 暂停累计时间（毫秒）
    paused_duration: Arc<AtomicU64>,
    /// 时间解析器
    time_parser: Arc<TimeParser>,
}

impl CountdownManager {
    /// 创建新的倒计时管理器
    pub async fn new() -> Result<Self> {
        let (update_sender, _) = broadcast::channel(100);
        
        Ok(Self {
            id: Uuid::new_v4(),
            status: Arc::new(RwLock::new(CountdownStatus::Idle)),
            current_task: Arc::new(RwLock::new(None)),
            update_sender,
            cancel_sender: Arc::new(RwLock::new(None)),
            pause_notify: Arc::new(Notify::new()),
            is_paused: Arc::new(AtomicBool::new(false)),
            start_timestamp: Arc::new(AtomicU64::new(0)),
            paused_duration: Arc::new(AtomicU64::new(0)),
            time_parser: Arc::new(TimeParser::new()),
        })
    }
    
    /// 获取管理器ID
    pub fn get_id(&self) -> Uuid {
        self.id
    }
    
    /// 获取更新通知接收器
    /// 
    /// UI组件可以通过此接收器获取倒计时状态更新
    pub fn get_update_receiver(&self) -> broadcast::Receiver<CountdownUpdate> {
        self.update_sender.subscribe()
    }
    
    /// 获取当前倒计时状态
    pub async fn get_status(&self) -> CountdownStatus {
        self.status.read().await.clone()
    }
    
    /// 从任务数据开始倒计时
    /// 
    /// # 参数
    /// 
    /// * `task` - 任务数据
    pub async fn start_countdown_from_task(&self, task: TaskData) -> Result<()> {
        // 根据任务类型计算目标时间
        let target_time = match task.task_type {
            TaskType::Once => {
                task.target_time.ok_or_else(|| anyhow!("单次任务缺少目标时间"))?
            },
            TaskType::Daily => {
                let daily_time = task.daily_time.ok_or_else(|| anyhow!("每日任务缺少时间设置"))?;
                let now = Local::now();
                let today = now.date_naive();
                let target_datetime = today.and_time(daily_time);
                
                // 如果今天的时间已过，则设置为明天
                 if Local::now().naive_local() > target_datetime {
                     let tomorrow = today + chrono::Duration::days(1);
                     Local.from_local_datetime(&tomorrow.and_time(daily_time)).single().unwrap()
                 } else {
                     Local.from_local_datetime(&target_datetime).single().unwrap()
                 }
            }
        };
        
        // 保存任务数据
        *self.current_task.write().await = Some(task.clone());
        
        info!("开始倒计时任务: {:?} -> {}", task.task_type, target_time.format("%Y-%m-%d %H:%M:%S"));
        
        self.start_countdown_internal(target_time, Some(task)).await
    }
    
    /// 开始倒计时
    /// 
    /// # 参数
    /// 
    /// * `target_time` - 目标时间
    pub async fn start_countdown(&self, target_time: DateTime<Local>) -> Result<()> {
        self.start_countdown_internal(target_time, None).await
    }
    
    /// 内部倒计时启动方法
    async fn start_countdown_internal(&self, target_time: DateTime<Local>, _task: Option<TaskData>) -> Result<()> {
        // 检查目标时间是否有效
        let now = Local::now();
        if target_time <= now {
            return Err(anyhow!("目标时间必须在当前时间之后"));
        }
        
        // 取消之前的倒计时
        self.cancel_countdown().await?;
        
        // 重置状态
        self.is_paused.store(false, Ordering::Relaxed);
        self.start_timestamp.store(now.timestamp_millis() as u64, Ordering::Relaxed);
        self.paused_duration.store(0, Ordering::Relaxed);
        
        // 创建取消通道
        let (cancel_tx, mut cancel_rx) = mpsc::unbounded_channel();
        *self.cancel_sender.write().await = Some(cancel_tx);
        
        // 克隆必要的引用
        let status = Arc::clone(&self.status);
        let current_task = Arc::clone(&self.current_task);
        let update_sender = self.update_sender.clone();
        let pause_notify = Arc::clone(&self.pause_notify);
        let is_paused = Arc::clone(&self.is_paused);
        let paused_duration = Arc::clone(&self.paused_duration);
        
        info!("开始倒计时，目标时间: {}", target_time.format("%Y-%m-%d %H:%M:%S"));
        
        // 启动倒计时任务
        tokio::spawn(async move {
            let mut interval = interval(tokio::time::Duration::from_secs(1));
            let mut pause_start: Option<Instant> = None;
            
            loop {
                // 检查是否收到取消信号
                if cancel_rx.try_recv().is_ok() {
                    info!("倒计时被取消");
                    *status.write().await = CountdownStatus::Cancelled;
                    info!("发送倒计时取消通知");
                    if let Err(e) = update_sender.send(CountdownUpdate::Cancelled) {
                        error!("发送倒计时取消通知失败: {:?}", e);
                    }
                    return;
                }
                
                // 检查暂停状态
                if is_paused.load(Ordering::Relaxed) {
                    if pause_start.is_none() {
                        pause_start = Some(Instant::now());
                        debug!("倒计时已暂停");
                    }
                    
                    // 等待恢复信号
                    pause_notify.notified().await;
                    
                    if let Some(start) = pause_start {
                        let pause_duration_ms = start.elapsed().as_millis() as u64;
                        paused_duration.fetch_add(pause_duration_ms, Ordering::Relaxed);
                        pause_start = None;
                        debug!("倒计时已恢复，暂停时长: {}ms", pause_duration_ms);
                    }
                    continue;
                }
                
                // 等待下一个tick
                interval.tick().await;
                
                // 计算剩余时间（考虑暂停时间）
                let now = Local::now();
                let total_paused_ms = paused_duration.load(Ordering::Relaxed);
                let adjusted_target = target_time + Duration::milliseconds(total_paused_ms as i64);
                let remaining = adjusted_target - now;
                
                if remaining.num_seconds() <= 0 {
                    // 倒计时结束
                    info!("倒计时结束");
                    *status.write().await = CountdownStatus::Finished;
                    
                    // 发送完成通知，包含任务信息
                    let task_info = current_task.read().await.clone();
                    info!("发送倒计时完成通知");
                    if let Err(e) = update_sender.send(CountdownUpdate::Finished) {
                        error!("发送倒计时完成通知失败: {:?}", e);
                    }
                    if let Some(task) = task_info {
                        info!("发送任务完成通知");
                        if let Err(e) = update_sender.send(CountdownUpdate::TaskCompleted { task_info: task }) {
                            error!("发送任务完成通知失败: {:?}", e);
                        }
                    }
                    return;
                } else {
                    // 更新状态
                    *status.write().await = CountdownStatus::Running { remaining };
                    
                    // 发送进度更新
                    let start_time = Local::now() - (adjusted_target - target_time);
                    let progress = Self::calculate_progress(start_time, adjusted_target, now);
                    debug!("发送倒计时进度更新: 剩余时间 {}秒, 进度 {:.1}%", remaining.num_seconds(), progress);
                    if let Err(e) = update_sender.send(CountdownUpdate::Progress { remaining, progress }) {
                        error!("发送倒计时进度更新失败: {:?}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// 取消当前倒计时
    pub async fn cancel_countdown(&self) -> Result<()> {
        // 发送取消信号
        if let Some(cancel_sender) = self.cancel_sender.read().await.as_ref() {
            let _ = cancel_sender.send(());
        }
        
        // 清除取消发送器
        *self.cancel_sender.write().await = None;
        
        // 更新状态
        *self.status.write().await = CountdownStatus::Cancelled;
        
        info!("倒计时已取消");
        Ok(())
    }
    
    /// 暂停倒计时
    pub async fn pause_countdown(&self) -> Result<()> {
        if self.is_active().await && !self.is_paused.load(Ordering::Relaxed) {
            self.is_paused.store(true, Ordering::Relaxed);
            debug!("倒计时已暂停");
            
            // 发送暂停状态更新
            info!("发送倒计时暂停通知");
            if let Err(e) = self.update_sender.send(CountdownUpdate::Paused) {
                error!("发送倒计时暂停通知失败: {:?}", e);
            }
        }
        Ok(())
    }
    
    /// 恢复倒计时
    pub async fn resume_countdown(&self) -> Result<()> {
        if self.is_active().await && self.is_paused.load(Ordering::Relaxed) {
            self.is_paused.store(false, Ordering::Relaxed);
            self.pause_notify.notify_one();
            debug!("倒计时已恢复");
            
            // 发送恢复状态更新
            info!("发送倒计时恢复通知");
            if let Err(e) = self.update_sender.send(CountdownUpdate::Resumed) {
                error!("发送倒计时恢复通知失败: {:?}", e);
            }
        }
        Ok(())
    }
    
    /// 重置倒计时管理器
    /// 
    /// 取消当前倒计时并重置状态为空闲
    pub async fn reset(&self) -> Result<()> {
        self.cancel_countdown().await?;
        *self.status.write().await = CountdownStatus::Idle;
        info!("倒计时管理器已重置");
        Ok(())
    }
    
    /// 检查是否有活动的倒计时
    pub async fn is_active(&self) -> bool {
        matches!(
            *self.status.read().await,
            CountdownStatus::Running { .. }
        )
    }
    
    /// 检查倒计时是否已暂停
    pub async fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }
    
    /// 获取当前任务信息
    pub async fn get_current_task(&self) -> Option<TaskData> {
        self.current_task.read().await.clone()
    }
    
    /// 获取倒计时开始时间戳（毫秒）
    pub fn get_start_timestamp(&self) -> Option<u64> {
        let timestamp = self.start_timestamp.load(Ordering::Relaxed);
        if timestamp > 0 {
            Some(timestamp)
        } else {
            None
        }
    }
    
    /// 获取总暂停时长（毫秒）
    pub fn get_total_paused_duration(&self) -> u64 {
        self.paused_duration.load(Ordering::Relaxed)
    }
    
    /// 获取剩余时间
    /// 
    /// 如果当前没有活动的倒计时，返回None
    pub async fn get_remaining_time(&self) -> Option<Duration> {
        match *self.status.read().await {
            CountdownStatus::Running { remaining } => Some(remaining),
            _ => None,
        }
    }
    
    /// 设置错误状态
    /// 
    /// 用于在发生错误时更新倒计时状态
    pub async fn set_error(&self, error_msg: String) {
        error!("倒计时错误: {}", error_msg);
        *self.status.write().await = CountdownStatus::Error(error_msg.clone());
        info!("发送倒计时错误通知: {}", error_msg);
        if let Err(e) = self.update_sender.send(CountdownUpdate::Error(error_msg)) {
            error!("发送倒计时错误通知失败: {:?}", e);
        }
    }
    
    /// 格式化剩余时间为用户友好的字符串
    /// 
    /// # 参数
    /// 
    /// * `duration` - 时间间隔
    pub fn format_duration(duration: &Duration) -> String {
        let total_seconds = duration.num_seconds();
        
        if total_seconds <= 0 {
            return "00:00:00".to_string();
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
    
    /// 计算进度百分比
    /// 
    /// # 参数
    /// 
    /// * `start_time` - 开始时间
    /// * `target_time` - 目标时间
    /// * `current_time` - 当前时间
    pub fn calculate_progress(
        start_time: DateTime<Local>,
        target_time: DateTime<Local>,
        current_time: DateTime<Local>,
    ) -> f64 {
        let total_duration = target_time - start_time;
        let elapsed_duration = current_time - start_time;
        
        if total_duration.num_seconds() <= 0 {
            return 100.0;
        }
        
        let progress = elapsed_duration.num_seconds() as f64 / total_duration.num_seconds() as f64;
        (progress * 100.0).min(100.0).max(0.0)
    }
}

/// 倒计时事件处理器
/// 
/// 用于处理倒计时相关的事件和回调
pub struct CountdownEventHandler {
    /// 倒计时完成回调
    on_finished: Option<Box<dyn Fn() + Send + Sync>>,
    /// 倒计时取消回调
    on_cancelled: Option<Box<dyn Fn() + Send + Sync>>,
    /// 倒计时错误回调
    on_error: Option<Box<dyn Fn(String) + Send + Sync>>,
}

impl CountdownEventHandler {
    /// 创建新的事件处理器
    pub fn new() -> Self {
        Self {
            on_finished: None,
            on_cancelled: None,
            on_error: None,
        }
    }
    
    /// 设置倒计时完成回调
    pub fn on_finished<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_finished = Some(Box::new(callback));
        self
    }
    
    /// 设置倒计时取消回调
    pub fn on_cancelled<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_cancelled = Some(Box::new(callback));
        self
    }
    
    /// 设置倒计时错误回调
    pub fn on_error<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_error = Some(Box::new(callback));
        self
    }
    
    /// 处理倒计时更新
    pub fn handle_update(&self, update: &CountdownUpdate) {
        match update {
            CountdownUpdate::Finished => {
                if let Some(callback) = &self.on_finished {
                    callback();
                }
            },
            CountdownUpdate::Cancelled => {
                if let Some(callback) = &self.on_cancelled {
                    callback();
                }
            },
            CountdownUpdate::Error(msg) => {
                if let Some(callback) = &self.on_error {
                    callback(msg.clone());
                }
            },
            CountdownUpdate::Progress { .. } => {
                // Progress事件通常由UI处理，这里不需要特殊处理
            },
            CountdownUpdate::Paused => {
                // 暂停事件处理
            },
            CountdownUpdate::Resumed => {
                // 恢复事件处理
            },
            CountdownUpdate::TaskCompleted { .. } => {
                // 任务完成事件处理
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};
    
    #[tokio::test]
    async fn test_countdown_basic() {
        let manager = CountdownManager::new().await.unwrap();
        
        // 测试初始状态
        assert!(matches!(manager.get_status().await, CountdownStatus::Idle));
        assert!(!manager.is_active().await);
        
        // 测试开始倒计时
        let target = Local::now() + Duration::seconds(2);
        manager.start_countdown(target).await.unwrap();
        
        // 等待一小段时间
        sleep(TokioDuration::from_millis(100)).await;
        
        // 检查状态
        assert!(manager.is_active().await);
        assert!(manager.get_remaining_time().await.is_some());
    }
    
    #[tokio::test]
    async fn test_countdown_cancel() {
        let manager = CountdownManager::new().await.unwrap();
        
        // 开始倒计时
        let target = Local::now() + Duration::seconds(10);
        manager.start_countdown(target).await.unwrap();
        
        // 取消倒计时
        manager.cancel_countdown().await.unwrap();
        
        // 等待状态更新
        sleep(TokioDuration::from_millis(100)).await;
        
        // 检查状态
        assert!(!manager.is_active().await);
        assert!(matches!(manager.get_status().await, CountdownStatus::Cancelled));
    }
    
    #[test]
    fn test_format_duration() {
        let duration = Duration::seconds(3661); // 1小时1分1秒
        assert_eq!(CountdownManager::format_duration(&duration), "01:01:01");
        
        let duration = Duration::seconds(61); // 1分1秒
        assert_eq!(CountdownManager::format_duration(&duration), "01:01");
        
        let duration = Duration::seconds(0);
        assert_eq!(CountdownManager::format_duration(&duration), "00:00:00");
    }
}