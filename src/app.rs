//! 应用程序主模块
//! 
//! 负责协调各个子模块，管理应用程序的整体生命周期

use anyhow::Result;
use chrono::TimeZone;
use log::{info, error, warn};

use crate::core::{
    countdown::CountdownManager,
    persistence::TaskPersistence,
    shutdown::ShutdownExecutor,
    system_compat::SystemCompatibility,
    time_parser::TimeParser,
    types::{UIEvent, TaskType, TimeInput},
};
use crate::ui::UIManager;


/// 应用程序主结构体
/// 
/// 管理所有核心组件和它们之间的通信
pub struct App {
    /// 时间解析器
    time_parser: TimeParser,
    /// 倒计时管理器
    countdown_manager: CountdownManager,
    /// 关机执行器
    shutdown_executor: ShutdownExecutor,
    /// 任务持久化管理器
    task_persistence: TaskPersistence,
    /// 系统兼容性检查器
    system_compatibility: SystemCompatibility,
    /// UI管理器
    ui_manager: Option<UIManager>,
}

impl App {
    /// 创建新的应用实例
    /// 
    /// # 返回值
    /// 
    /// 返回初始化完成的应用实例或错误
    pub async fn new() -> Result<Self> {
        info!("初始化应用组件...");

        // 检查系统兼容性
        let mut system_compatibility = SystemCompatibility::new();
        system_compatibility.initialize().await?;
        
        // 生成兼容性报告
        let compat_report = system_compatibility.generate_compatibility_report();
        info!("系统兼容性报告:\n{}", compat_report);
        
        // 检查是否支持关机功能
        if !system_compatibility.is_shutdown_supported() {
            return Err(anyhow::anyhow!("当前系统不支持定时关机功能"));
        }
        
        // 检查管理员权限
        if system_compatibility.requires_admin_privileges() && !system_compatibility.has_admin_privileges() {
            warn!("当前程序没有管理员权限，可能无法执行关机操作");
        }

        // 初始化核心组件
        let time_parser = TimeParser::new();
        let countdown_manager = CountdownManager::new().await?;
        let shutdown_executor = ShutdownExecutor::new().await?;
        let task_persistence = TaskPersistence::new()?;

        // 尝试恢复之前的任务
        let app = Self {
            time_parser,
            countdown_manager,
            shutdown_executor,
            task_persistence,
            system_compatibility,
            ui_manager: None,
        };
        
        if let Ok(Some(task)) = app.task_persistence.load_task() {
            info!("发现已保存的任务，尝试恢复: {:?}", task.task_type);
            
            // 检查任务是否仍然有效
            if let Some(target_time) = task.target_time {
                let now = chrono::Local::now();
                
                if target_time > now {
                    // 任务仍然有效，恢复倒计时
                    if let Err(e) = app.countdown_manager.start_countdown(target_time).await {
                        error!("恢复倒计时失败: {}", e);
                        // 清除无效任务
                        let _ = app.task_persistence.clear_task();
                    } else {
                        info!("任务已恢复，目标时间: {:?}", target_time);
                    }
                } else {
                    // 任务已过期，清除
                    info!("任务已过期，清除保存的任务数据");
                    let _ = app.task_persistence.clear_task();
                }
            }
        }
        
        return Ok(app);
    }

    /// 运行应用程序
    /// 
    /// 启动GUI界面并进入事件循环
    pub async fn run(self) -> Result<()> {
        info!("启动用户界面...");

        // 获取倒计时更新接收器
        let countdown_receiver = self.countdown_manager.get_update_receiver();
        info!("获取倒计时接收器成功");

        // 创建UI事件通道
        let (ui_event_sender, ui_event_receiver) = tokio::sync::mpsc::unbounded_channel::<UIEvent>();
        info!("创建UI事件通道成功");

        // 创建一个共享的倒计时管理器引用
        let countdown_manager = std::sync::Arc::new(tokio::sync::Mutex::new(self.countdown_manager));
        let time_parser = self.time_parser.clone();
        let shutdown_executor = std::sync::Arc::new(tokio::sync::Mutex::new(self.shutdown_executor));
        
        // 启动UI事件处理任务
        let countdown_manager_clone = countdown_manager.clone();
        let shutdown_executor_clone = shutdown_executor.clone();
        tokio::spawn(async move {
            info!("启动UI事件处理循环");
            let mut ui_event_receiver = ui_event_receiver;
            while let Some(event) = ui_event_receiver.recv().await {
                info!("收到UI事件: {:?}", event);
                match event {
                    UIEvent::StartCountdown(time_input, task_type) => {
                         info!("处理开始倒计时事件: {:?}", time_input);
                         let target_time = match time_input {
                             TimeInput::Duration(duration) => chrono::Local::now() + duration,
                             TimeInput::AbsoluteTime(datetime) => datetime,
                             TimeInput::DailyTime(time) => {
                                 // 将每日时间转换为今天的绝对时间
                                 let today = chrono::Local::now().date_naive();
                                 today.and_time(time).and_local_timezone(chrono::Local).unwrap()
                             },
                         };
                        let countdown_manager = countdown_manager_clone.lock().await;
                        if let Err(e) = countdown_manager.start_countdown(target_time).await {
                            error!("启动倒计时失败: {}", e);
                        }
                    },
                    UIEvent::CancelCountdown => {
                        info!("处理取消倒计时事件");
                        let countdown_manager = countdown_manager_clone.lock().await;
                        if let Err(e) = countdown_manager.cancel_countdown().await {
                            error!("取消倒计时失败: {}", e);
                        }
                    },
                    UIEvent::QuickCountdown(seconds) => {
                        info!("处理快速倒计时事件: {} 秒", seconds);
                        let duration = chrono::Duration::seconds(seconds as i64);
                        let target_time = chrono::Local::now() + duration;
                        let countdown_manager = countdown_manager_clone.lock().await;
                        if let Err(e) = countdown_manager.start_countdown(target_time).await {
                            error!("启动快速倒计时失败: {}", e);
                        }
                    },
                    _ => {
                        info!("处理其他UI事件: {:?}", event);
                    }
                }
            }
            info!("UI事件处理循环结束");
        });

        // 启动GUI事件循环，传递必要的参数
        crate::ui::manager::run_with_params(
            time_parser,
            Some(countdown_receiver),
            Some(ui_event_sender),
        )?;

        Ok(())
    }

    /// 处理用户输入的时间设置
    /// 
    /// # 参数
    /// 
    /// * `input` - 用户输入的时间字符串
    /// * `task_type` - 任务类型（单次或每日）
    pub async fn set_shutdown_time(&mut self, input: &str, task_type: crate::core::types::TaskType) -> Result<()> {
        // 解析时间输入
        let time_input = self.time_parser.parse(input)?;
        
        // 验证时间有效性
        self.time_parser.validate(&time_input)?;

        // 启动倒计时
        let target_time = match time_input {
            crate::core::types::TimeInput::Duration(duration) => {
                chrono::Local::now() + duration
            },
            crate::core::types::TimeInput::AbsoluteTime(datetime) => datetime,
            crate::core::types::TimeInput::DailyTime(time) => {
                // 计算下一个匹配的日期时间
                let now = chrono::Local::now();
                let today = now.date_naive();
                let target_datetime = today.and_time(time);
                
                if target_datetime > now.naive_local() {
                    chrono::Local.from_local_datetime(&target_datetime).unwrap()
                } else {
                    // 如果今天的时间已过，设置为明天
                    let tomorrow = today + chrono::Duration::days(1);
                    chrono::Local.from_local_datetime(&tomorrow.and_time(time)).unwrap()
                }
            }
        };

        // 保存任务
        let task_data = crate::core::types::TaskData {
            task_type,
            target_time: Some(target_time),
            daily_time: if let crate::core::types::TimeInput::DailyTime(time) = time_input {
                Some(time)
            } else {
                None
            },
            enabled: true,
            created_at: chrono::Local::now(),
        };
        
        self.task_persistence.save_task(&task_data)?;

        // 启动倒计时
        self.countdown_manager.start_countdown(target_time).await?;

        info!("定时关机任务已设置: {:?}", target_time);
        Ok(())
    }

    /// 取消当前的关机任务
    pub async fn cancel_shutdown(&mut self) -> Result<()> {
        self.countdown_manager.cancel_countdown().await?;
        self.task_persistence.clear_task()?;
        info!("关机任务已取消");
        Ok(())
    }

    /// 执行关机操作
    pub async fn execute_shutdown(&self) -> Result<()> {
        info!("执行关机操作...");
        self.shutdown_executor.shutdown(0).await?;
        Ok(())
    }

    /// 处理UI事件
    /// 
    /// # 参数
    /// 
    /// * `event` - UI事件
    pub async fn handle_ui_event(&mut self, event: UIEvent) -> Result<()> {
        match event {
            UIEvent::StartCountdown(time_input, task_type) => {
                info!("收到开始倒计时事件: {:?}", time_input);
                self.start_countdown_from_input(time_input, task_type).await?;
            },
            UIEvent::CancelCountdown => {
                info!("收到取消倒计时事件");
                self.cancel_shutdown().await?;
            },
            UIEvent::QuickCountdown(seconds) => {
                info!("收到快速倒计时事件: {} 秒", seconds);
                let duration = chrono::Duration::seconds(seconds as i64);
                let target_time = chrono::Local::now() + duration;
                self.countdown_manager.start_countdown(target_time).await?;
            },
            UIEvent::MinimizeToTray => {
                info!("最小化到托盘");
                // 这里可以添加最小化逻辑
            },
            UIEvent::RestoreFromTray => {
                info!("从托盘恢复");
                // 这里可以添加恢复逻辑
            },
            UIEvent::ShowMainWindow => {
                info!("显示主窗口");
                // 这里可以添加显示主窗口逻辑
            },
            UIEvent::ToggleMainWindow => {
                info!("切换主窗口显示状态");
                // 这里可以添加切换逻辑
            },
            UIEvent::ShowSettings => {
                info!("显示设置窗口");
                // 这里可以添加设置窗口逻辑
            },
            UIEvent::ShowAbout => {
                info!("显示关于窗口");
                // 这里可以添加关于窗口逻辑
            },
            UIEvent::Exit => {
                info!("退出应用程序");
                std::process::exit(0);
            },
        }
        Ok(())
    }

    /// 从时间输入启动倒计时
    /// 
    /// # 参数
    /// 
    /// * `time_input` - 时间输入
    /// * `task_type` - 任务类型
    async fn start_countdown_from_input(&mut self, time_input: TimeInput, task_type: TaskType) -> Result<()> {
        // 计算目标时间
        let target_time = match time_input {
            TimeInput::Duration(duration) => {
                chrono::Local::now() + duration
            },
            TimeInput::AbsoluteTime(datetime) => datetime,
            TimeInput::DailyTime(time) => {
                // 计算下一个匹配的日期时间
                let now = chrono::Local::now();
                let today = now.date_naive();
                let target_datetime = today.and_time(time);
                
                if target_datetime > now.naive_local() {
                    chrono::Local.from_local_datetime(&target_datetime).unwrap()
                } else {
                    // 如果今天的时间已过，设置为明天
                    let tomorrow = today + chrono::Duration::days(1);
                    chrono::Local.from_local_datetime(&tomorrow.and_time(time)).unwrap()
                }
            }
        };

        // 保存任务
        let task_data = crate::core::types::TaskData {
            task_type,
            target_time: Some(target_time),
            daily_time: if let TimeInput::DailyTime(time) = time_input {
                Some(time)
            } else {
                None
            },
            enabled: true,
            created_at: chrono::Local::now(),
        };
        
        self.task_persistence.save_task(&task_data)?;

        // 启动倒计时
        self.countdown_manager.start_countdown(target_time).await?;

        info!("定时关机任务已设置: {:?}", target_time);
        Ok(())
    }
}