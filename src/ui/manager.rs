//! UI管理器模块
//! 
//! 负责管理整个用户界面，使用iced框架实现跨平台GUI

use anyhow::Result;
use iced::{
    widget::{button, column, container, row, text, text_input, Space},
    Application, Command, Element, Length, Settings, Theme as IcedTheme, executor, Font, time, window,
};
use iced::widget::container::Appearance as ContainerAppearance;
use iced::{Background, Border, Color};
use log::{info, error};
use tokio::sync::{mpsc, broadcast};

use crate::core::{
    time_parser::TimeParser,
    types::{CountdownUpdate, CountdownStatus, UIEvent, TaskType, TimeInput},
};
use crate::ui::{
    tray::TrayManager,
    theme::Theme,
};

/// 应用程序消息类型
/// 
/// 定义了应用程序中所有可能的用户交互和系统事件
#[derive(Debug, Clone)]
pub enum Message {
    /// 时间输入改变
    TimeInputChanged(String),
    /// 更新时间输入
    UpdateTimeInput(TimeInput),
    /// 更新任务类型
    UpdateTaskType(TaskType),
    /// 开始倒计时
    StartCountdown,
    /// 取消倒计时
    CancelCountdown,
    /// 最小化到托盘
    MinimizeToTray,
    /// 从托盘恢复
    RestoreFromTray,
    /// 切换主题
    ToggleTheme,
    /// 退出应用
    Exit,
    /// 倒计时更新
    CountdownUpdate(CountdownUpdate),
    /// 快速倒计时
    QuickCountdown(u32),
    /// 显示设置
    ShowSettings,
    /// 显示关于
    ShowAbout,
    /// 检查倒计时状态
    CheckCountdownStatus,
}

/// UI管理器应用程序状态
/// 
/// 使用iced的Application trait实现GUI应用程序
#[derive(Debug)]
pub struct UIManager {
    /// 时间输入字符串
    time_input: String,
    /// 当前倒计时状态
    countdown_status: CountdownStatus,
    /// 时间解析器
    time_parser: TimeParser,
    /// 托盘管理器
    tray_manager: Option<TrayManager>,
    /// UI事件发送器
    ui_event_sender: Option<mpsc::UnboundedSender<UIEvent>>,
    /// 倒计时更新接收器
    countdown_receiver: Option<broadcast::Receiver<CountdownUpdate>>,
    /// 当前主题
    theme: Theme,
    /// 是否使用暗色主题
    is_dark_theme: bool,
    /// 是否最小化到托盘
    minimized_to_tray: bool,
    /// 是否显示设置窗口
    show_settings: bool,
    /// 是否显示关于窗口
    show_about: bool,
}

impl UIManager {
    /// 创建新的UI管理器
    /// 
    /// # 参数
    /// 
    /// * `time_parser` - 时间解析器
    /// * `countdown_receiver` - 倒计时更新接收器
    /// * `ui_event_sender` - UI事件发送器
    pub async fn new(
        time_parser: TimeParser,
        countdown_receiver: Option<broadcast::Receiver<CountdownUpdate>>,
        ui_event_sender: Option<mpsc::UnboundedSender<UIEvent>>,
    ) -> Result<Self> {
        info!("初始化UI管理器...");
        
        // 创建托盘管理器
        let tray_manager = if let Some(sender) = &ui_event_sender {
            let mut tray_manager_instance = TrayManager::new(sender.clone());
            match tray_manager_instance.initialize() {
                Ok(_) => {
                    info!("托盘图标创建成功");
                    Some(tray_manager_instance)
                },
                Err(e) => {
                    error!("创建托盘图标失败: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            time_input: String::new(),
            countdown_status: CountdownStatus::Idle,
            time_parser,
            tray_manager,
            ui_event_sender,
            countdown_receiver,
            theme: Theme::default(),
            is_dark_theme: false,
            minimized_to_tray: false,
            show_settings: false,
            show_about: false,
        })
    }
    
    /// 处理倒计时更新
    /// 
    /// # 参数
    /// 
    /// * `update` - 倒计时更新消息
    fn handle_countdown_update(&mut self, update: CountdownUpdate) {
        info!("收到倒计时更新: {:?}", update);
        match update {
            CountdownUpdate::Progress { remaining, progress: _ } => {
                info!("倒计时进度更新 - 剩余时间: {:?}", remaining);
                self.countdown_status = CountdownStatus::Running { remaining };
            },
            CountdownUpdate::Finished => {
                info!("倒计时完成");
                self.countdown_status = CountdownStatus::Finished;
                self.show_shutdown_notification();
            },
            CountdownUpdate::Cancelled => {
                info!("倒计时被取消");
                self.countdown_status = CountdownStatus::Cancelled;
            },
            CountdownUpdate::Error(msg) => {
                error!("倒计时错误: {}", msg);
                self.countdown_status = CountdownStatus::Error(msg.clone());
                self.show_error_notification(&msg);
            },
            CountdownUpdate::Paused => {
                info!("倒计时已暂停");
            },
            CountdownUpdate::Resumed => {
                info!("倒计时已恢复");
            },
            CountdownUpdate::TaskCompleted { task_info: _ } => {
                info!("任务已完成");
            }
        }
    }
    
    /// 显示关机通知
    fn show_shutdown_notification(&self) {
        info!("显示关机通知");
        if let Some(tray) = &self.tray_manager {
            tray.show_notification("QtShut", "倒计时结束，即将关机");
        }
    }
    
    /// 显示错误通知
    /// 
    /// # 参数
    /// 
    /// * `message` - 错误消息
    fn show_error_notification(&self, message: &str) {
        error!("显示错误通知: {}", message);
        if let Some(tray) = &self.tray_manager {
            tray.show_notification("QtShut - 错误", message);
        }
    }
    
    /// 发送UI事件
    /// 
    /// # 参数
    /// 
    /// * `event` - UI事件
    fn send_ui_event(&self, event: UIEvent) {
        if let Some(sender) = &self.ui_event_sender {
            if let Err(e) = sender.send(event) {
                error!("发送UI事件失败: {}", e);
            }
        }
    }
}

/// 运行UI应用程序
/// 
/// # 参数
/// 
/// * `time_parser` - 时间解析器
/// * `countdown_receiver` - 倒计时更新接收器
/// * `ui_event_sender` - UI事件发送器
/// 
/// # 返回值
/// 
/// 返回iced应用程序的运行结果
pub fn run_with_params(
    time_parser: TimeParser,
    countdown_receiver: Option<broadcast::Receiver<CountdownUpdate>>,
    ui_event_sender: Option<mpsc::UnboundedSender<UIEvent>>,
) -> iced::Result {
    let flags = (time_parser, countdown_receiver, ui_event_sender);
    let settings = Settings {
        id: None,
        window: window::Settings {
            size: iced::Size::new(400.0, 500.0),
            position: window::Position::default(),
            min_size: None,
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            level: window::Level::Normal,
            icon: None,
            platform_specific: Default::default(),
            exit_on_close_request: true,
        },
        flags,
        fonts: vec![],
        default_font: Font::with_name("Microsoft YaHei"),
        default_text_size: iced::Pixels(16.0),
        antialiasing: false,
    };
    UIManager::run(settings)
}

/// 运行UI应用程序（兼容性函数）
/// 
/// 启动iced应用程序的主循环
pub fn run() -> iced::Result {
    let time_parser = TimeParser::new();
    run_with_params(time_parser, None, None)
}

impl Application for UIManager {
    type Message = Message;
    type Theme = IcedTheme;
    type Executor = executor::Default;
    type Flags = (TimeParser, Option<broadcast::Receiver<CountdownUpdate>>, Option<mpsc::UnboundedSender<UIEvent>>);
    
    /// 订阅外部事件
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        // 创建一个定时器来定期检查倒计时状态
        iced::time::every(std::time::Duration::from_millis(500))
            .map(|_| Message::CheckCountdownStatus)
    }

    /// 创建应用程序实例
    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let (time_parser, countdown_receiver, ui_event_sender) = flags;
        info!("创建UIManager实例，倒计时接收器: {}", if countdown_receiver.is_some() { "已设置" } else { "未设置" });
        
        let ui_manager = Self {
            time_input: String::new(),
            countdown_status: CountdownStatus::Idle,
            time_parser,
            tray_manager: None,
            ui_event_sender,
            countdown_receiver,
            theme: Theme::default(),
            is_dark_theme: false,
            minimized_to_tray: false,
            show_settings: false,
            show_about: false,
        };
        
        (ui_manager, Command::none())
    }

    /// 应用程序标题
    fn title(&self) -> String {
        "QtShut - 定时关机".to_string()
    }

    /// 处理消息更新
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::TimeInputChanged(input) => {
                self.time_input = input;
                Command::none()
            },
            Message::UpdateTimeInput(_time_input) => {
                // 更新时间输入类型
                // 这里可以根据需要更新UI状态
                Command::none()
            },
            Message::UpdateTaskType(_task_type) => {
                // 更新任务类型
                // 这里可以根据需要更新UI状态
                Command::none()
            },
            Message::StartCountdown => {
                info!("用户请求开始倒计时，当前输入: '{}'", self.time_input);
                
                // 解析时间输入
                match self.time_parser.parse(&self.time_input) {
                    Ok(time_input) => {
                        info!("时间解析成功: {:?}", time_input);
                        info!("发送StartCountdown事件到应用层");
                        self.send_ui_event(UIEvent::StartCountdown(time_input, TaskType::Once));
                        self.countdown_status = CountdownStatus::Running { 
                            remaining: chrono::Duration::seconds(0) // 临时值，会被实际倒计时更新
                        };
                    },
                    Err(e) => {
                        error!("时间解析失败: {}", e);
                        self.countdown_status = CountdownStatus::Error(format!("时间格式错误: {}", e));
                    }
                }
                Command::none()
            },
            Message::CancelCountdown => {
                info!("用户请求取消倒计时");
                self.send_ui_event(UIEvent::CancelCountdown);
                self.countdown_status = CountdownStatus::Cancelled;
                Command::none()
            },
            Message::MinimizeToTray => {
                info!("最小化窗口");
                self.send_ui_event(UIEvent::MinimizeToTray);
                self.minimized_to_tray = true;
                window::minimize(window::Id::MAIN, true)
            },
            Message::RestoreFromTray => {
                self.send_ui_event(UIEvent::RestoreFromTray);
                self.minimized_to_tray = false;
                Command::none()
            },
            Message::ToggleTheme => {
                self.is_dark_theme = !self.is_dark_theme;
                self.theme = if self.is_dark_theme {
                    Theme::dark_theme()
                } else {
                    Theme::light_theme()
                };
                Command::none()
            },
            Message::Exit => {
                info!("用户请求退出应用程序");
                self.send_ui_event(UIEvent::Exit);
                std::process::exit(0);
            },
            Message::CountdownUpdate(update) => {
                self.handle_countdown_update(update);
                Command::none()
            },
            Message::QuickCountdown(minutes) => {
                info!("快速倒计时: {} 分钟", minutes);
                // 更新输入框显示
                self.time_input = format!("{}分钟", minutes);
                // 发送UI事件
                let seconds = minutes * 60;
                self.send_ui_event(UIEvent::QuickCountdown(seconds));
                Command::none()
            },
            Message::ShowSettings => {
                info!("显示设置窗口");
                self.show_settings = !self.show_settings;
                self.send_ui_event(UIEvent::ShowSettings);
                Command::none()
            },
            Message::ShowAbout => {
                info!("显示关于窗口");
                self.show_about = !self.show_about;
                self.send_ui_event(UIEvent::ShowAbout);
                Command::none()
            },
            Message::CheckCountdownStatus => {
                // 检查是否有倒计时更新
                let mut updates = Vec::new();
                let mut message_count = 0;
                if let Some(ref mut receiver) = self.countdown_receiver {
                    loop {
                        match receiver.try_recv() {
                            Ok(update) => {
                                message_count += 1;
                                updates.push(update);
                            },
                            Err(broadcast::error::TryRecvError::Empty) => {
                                // 没有更多消息，退出循环
                                break;
                            },
                            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                                // 消息滞后，继续尝试接收
                                info!("倒计时消息滞后，跳过了{}条消息", skipped);
                                continue;
                            },
                            Err(broadcast::error::TryRecvError::Closed) => {
                                // 通道已关闭，退出循环
                                error!("倒计时通道已关闭");
                                break;
                            },
                        }
                    }
                } else {
                    info!("倒计时接收器为空");
                }
                
                if message_count > 0 {
                    info!("检查到{}条倒计时更新消息", message_count);
                } else {
                    info!("检查倒计时状态 - 无新消息");
                }
                
                // 处理收集到的更新
                for update in updates {
                    self.handle_countdown_update(update);
                }
                Command::none()
            },
        }
    }

    /// 构建用户界面
    fn view(&self) -> Element<Self::Message> {
        let title = text("QtShut - 定时关机")
            .size(24)
            .width(Length::Fill);

        let time_input = text_input(
            "请输入时间 (如: 30分钟, 1小时, 22:30)",
            &self.time_input,
        )
        .on_input(Message::TimeInputChanged)
        .padding(10)
        .size(16)
        .width(Length::Fixed(300.0));

        let start_button = button("开始倒计时")
            .on_press(Message::StartCountdown)
            .padding(10);

        let cancel_button = button("取消倒计时")
            .on_press(Message::CancelCountdown)
            .padding(10);

        let button_row = row![
            start_button,
            Space::with_width(10),
            cancel_button,
        ]
        .spacing(10);

        // 显示倒计时状态
        let status_text = match &self.countdown_status {
            CountdownStatus::Idle => "等待开始...".to_string(),
            CountdownStatus::Running { remaining } => {
                format!("剩余时间: {}小时{}分钟{}秒", 
                    remaining.num_hours(),
                    remaining.num_minutes() % 60,
                    remaining.num_seconds() % 60
                )
            },
            CountdownStatus::Finished => "倒计时结束！".to_string(),
            CountdownStatus::Cancelled => "倒计时已取消".to_string(),
            CountdownStatus::Error(msg) => format!("错误: {}", msg),
        };

        let status_display = text(status_text)
            .size(18)
            .width(Length::Fill);

        // 快速倒计时按钮
        let quick_buttons = row![
            button("5分钟").on_press(Message::QuickCountdown(5)),
            Space::with_width(5),
            button("10分钟").on_press(Message::QuickCountdown(10)),
            Space::with_width(5),
            button("30分钟").on_press(Message::QuickCountdown(30)),
            Space::with_width(5),
            button("1小时").on_press(Message::QuickCountdown(60)),
        ]
        .spacing(5);

        // 控制按钮
        let control_buttons = row![
            button("设置").on_press(Message::ShowSettings),
            Space::with_width(5),
            button("关于").on_press(Message::ShowAbout),
            Space::with_width(5),
            button("切换主题").on_press(Message::ToggleTheme),
            Space::with_width(5),
            button("最小化").on_press(Message::MinimizeToTray),
        ]
        .spacing(5);

        let content = column![
            title,
            Space::with_height(20),
            time_input,
            Space::with_height(15),
            button_row,
            Space::with_height(20),
            status_display,
            Space::with_height(20),
            text("快速倒计时:").size(16),
            Space::with_height(10),
            quick_buttons,
            Space::with_height(20),
            control_buttons,
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        let main_content = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y();

        // 如果显示设置窗口
        if self.show_settings {
            let settings_content = column![
                text("设置").size(24),
                Space::with_height(20),
                text("主题设置:"),
                button(if self.is_dark_theme { "切换到浅色主题" } else { "切换到深色主题" })
                    .on_press(Message::ToggleTheme),
                Space::with_height(20),
                button("关闭").on_press(Message::ShowSettings),
            ]
            .spacing(10)
            .padding(20)
            .width(Length::Fixed(300.0));

            let settings_modal = container(settings_content)
                 .style(ContainerAppearance {
                     background: Some(Background::Color(Color::WHITE)),
                     border: Border {
                         color: Color::BLACK,
                         width: 2.0,
                         radius: 10.0.into(),
                     },
                     ..Default::default()
                 })
                .center_x()
                .center_y();

            return settings_modal.into();
        }

        // 如果显示关于窗口
        if self.show_about {
            let about_content = column![
                text("关于 QtShut").size(24),
                Space::with_height(20),
                text("版本: 1.0.0"),
                text("一个简单的定时关机工具"),
                text("使用 Rust + Iced 开发"),
                Space::with_height(20),
                button("关闭").on_press(Message::ShowAbout),
            ]
            .spacing(10)
            .padding(20)
            .width(Length::Fixed(300.0));

            let about_modal = container(about_content)
                 .style(ContainerAppearance {
                     background: Some(Background::Color(Color::WHITE)),
                     border: Border {
                         color: Color::BLACK,
                         width: 2.0,
                         radius: 10.0.into(),
                     },
                     ..Default::default()
                 })
                .center_x()
                .center_y();

            return about_modal.into();
        }

        main_content.into()
    }

    /// 应用程序主题
    fn theme(&self) -> Self::Theme {
        if self.is_dark_theme {
            IcedTheme::Dark
        } else {
            IcedTheme::Light
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::time_parser::TimeParser;
    
    #[tokio::test]
    async fn test_ui_manager_creation() {
        let time_parser = TimeParser::new();
        let ui_manager = UIManager::new(time_parser, None, None).await;
        assert!(ui_manager.is_ok());
    }
    
    #[test]
    fn test_message_handling() {
        let time_parser = TimeParser::new();
        let mut ui_manager = UIManager {
            time_input: String::new(),
            countdown_status: CountdownStatus::Idle,
            time_parser,
            tray_manager: None,
            ui_event_sender: None,
            countdown_receiver: None,
            show_settings: false,
            show_about: false,
            theme: Theme::default(),
            is_dark_theme: false,
            minimized_to_tray: false,
        };
        
        // 测试时间输入消息
        let _command = ui_manager.update(Message::TimeInputChanged("30分钟".to_string()));
        assert_eq!(ui_manager.time_input, "30分钟");
        
        // 测试主题切换
        let _command = ui_manager.update(Message::ToggleTheme);
        assert!(ui_manager.is_dark_theme);
    }
}