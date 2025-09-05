//! UI组件模块
//! 
//! 提供应用程序的各种UI组件，包括主窗口、时间输入面板、倒计时显示等

use iced::widget::{button, column, container, row, text, text_input, pick_list, Space};
use iced::{Element, Length, Color, Background, Alignment, Theme as IcedTheme, Border, Shadow};
use chrono::Duration;
use crate::core::types::{TimeInput, TaskType};
use crate::ui::theme::Theme;
use crate::ui::manager::Message;

/// 主窗口状态
/// 
/// 管理主窗口的显示状态和用户交互
#[derive(Debug, Clone)]
pub struct MainWindowState {
    /// 时间输入
    pub time_input: TimeInput,
    /// 任务类型
    pub task_type: TaskType,
    /// 是否正在倒计时
    pub is_counting: bool,
    /// 剩余时间（秒）
    pub remaining_seconds: Option<u64>,
    /// 应用主题
    pub theme: Theme,
}

impl Default for MainWindowState {
    fn default() -> Self {
        Self {
            time_input: TimeInput::default(),
            task_type: TaskType::Once,
            is_counting: false,
            remaining_seconds: None,
            theme: Theme::default(),
        }
    }
}

impl MainWindowState {
    /// 创建新的主窗口状态
    /// 
    /// # 返回值
    /// 
    /// 返回默认的主窗口状态
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 构建主窗口视图
    /// 
    /// # 返回值
    /// 
    /// 返回主窗口的Element
    pub fn view(&self) -> Element<Message> {
        let content = column![
            self.build_header(),
            Space::with_height(Length::Fixed(20.0)),
            self.build_time_input_panel(),
            Space::with_height(Length::Fixed(20.0)),
            self.build_countdown_display(),
            Space::with_height(Length::Fixed(20.0)),
            self.build_control_panel(),
        ]
        .spacing(10)
        .padding(20)
        .align_items(Alignment::Center);
        
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(|_theme: &IcedTheme| {
                container::Appearance {
                    background: Some(Background::Color(Color::from_rgb8(248, 249, 250))),
                    border: Border::default(),
                    shadow: Shadow::default(),
                    text_color: None,
                }
            })
            .into()
    }
    
    /// 构建标题栏
    /// 
    /// # 返回值
    /// 
    /// 返回标题栏的Element
    fn build_header(&self) -> Element<Message> {
        text("定时关机工具")
            .size(24)
            .style(Color::from_rgb8(51, 51, 51))
            .into()
    }
    
    /// 构建时间输入面板
    /// 
    /// # 返回值
    /// 
    /// 返回时间输入面板的Element
    fn build_time_input_panel(&self) -> Element<Message> {
        let hours_input = text_input("小时", "0")
            .on_input(|value| {
                if let Ok(_hours) = value.parse::<u32>() {
                    Message::UpdateTimeInput(TimeInput::default())
                } else {
                    Message::UpdateTimeInput(TimeInput::default())
                }
            })
            .padding(8)
            .size(14)
            .width(Length::Fixed(80.0));
            
        let minutes_input = text_input("分钟", "30")
            .on_input(|value| {
                if let Ok(minutes) = value.parse::<u32>() {
                    Message::UpdateTimeInput(TimeInput::Duration(Duration::minutes(minutes as i64)))
                } else {
                    Message::UpdateTimeInput(TimeInput::default())
                }
            })
            .padding(8)
            .size(14)
            .width(Length::Fixed(80.0));
            
        let seconds_input = text_input("秒", "0")
            .on_input(|value| {
                if let Ok(_seconds) = value.parse::<u32>() {
                    Message::UpdateTimeInput(TimeInput::default())
                } else {
                    Message::UpdateTimeInput(TimeInput::default())
                }
            })
            .padding(8)
            .size(14)
            .width(Length::Fixed(80.0));
            
        let task_type_picker = pick_list(
            vec![TaskType::Once, TaskType::Daily],
            Some(self.task_type),
            Message::UpdateTaskType,
        )
        .padding(8)
        .text_size(14)
        .width(Length::Fixed(120.0));
        
        column![
            text("设置时间")
                .size(16)
                .style(Color::from_rgb8(68, 68, 68)),
            Space::with_height(Length::Fixed(10.0)),
            row![
                hours_input,
                text("时").size(14),
                minutes_input,
                text("分").size(14),
                seconds_input,
                text("秒").size(14),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
            Space::with_height(Length::Fixed(10.0)),
            row![
                text("任务类型:").size(14),
                task_type_picker,
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        ]
        .spacing(5)
        .align_items(Alignment::Center)
        .into()
    }
    
    /// 构建倒计时显示
    /// 
    /// # 返回值
    /// 
    /// 返回倒计时显示的Element
    fn build_countdown_display(&self) -> Element<Message> {
        if let Some(remaining) = self.remaining_seconds {
            let hours = remaining / 3600;
            let minutes = (remaining % 3600) / 60;
            let seconds = remaining % 60;
            
            let status_text = if self.is_counting {
                format!("倒计时进行中: {:02}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                "倒计时已暂停".to_string()
            };
            
            let color = if self.is_counting {
                Color::from_rgb8(40, 167, 69)  // 绿色
            } else {
                Color::from_rgb8(255, 193, 7)  // 黄色
            };
            
            column![
                text(&status_text)
                    .size(18)
                    .style(color),
                text(format!("将执行: {:?}", self.task_type))
                    .size(14)
                    .style(Color::from_rgb8(108, 117, 125)),
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .into()
        } else {
            text("未设置倒计时")
                .size(16)
                .style(Color::from_rgb8(108, 117, 125))
                .into()
        }
    }
    
    /// 构建控制面板
    /// 
    /// # 返回值
    /// 
    /// 返回控制面板的Element
    fn build_control_panel(&self) -> Element<Message> {
        let start_button = button(
            text("开始倒计时")
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .size(14)
        )
        .on_press(Message::StartCountdown)
        .padding(12)
        .width(Length::Fixed(120.0))
        .style(iced::theme::Button::Primary);
        
        let cancel_button = button(
            text("取消倒计时")
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .size(14)
        )
        .on_press(Message::CancelCountdown)
        .padding(12)
        .width(Length::Fixed(120.0))
        .style(iced::theme::Button::Destructive);
        
        let quick_buttons = row![
            button(text("1分钟").size(12))
                .on_press(Message::QuickCountdown(1))
                .padding(8)
                .width(Length::Fixed(80.0))
                .style(iced::theme::Button::Secondary),
            button(text("5分钟").size(12))
                .on_press(Message::QuickCountdown(5))
                .padding(8)
                .width(Length::Fixed(80.0))
                .style(iced::theme::Button::Secondary),
            button(text("10分钟").size(12))
                .on_press(Message::QuickCountdown(10))
                .padding(8)
                .width(Length::Fixed(80.0))
                .style(iced::theme::Button::Secondary),
            button(text("30分钟").size(12))
                .on_press(Message::QuickCountdown(30))
                .padding(8)
                .width(Length::Fixed(80.0))
                .style(iced::theme::Button::Secondary),
        ]
        .spacing(8);
        
        column![
            row![start_button, cancel_button]
                .spacing(20)
                .align_items(Alignment::Center),
            Space::with_height(Length::Fixed(15.0)),
            text("快速设置:")
                .size(14)
                .style(Color::from_rgb8(108, 117, 125)),
            quick_buttons,
        ]
        .spacing(10)
        .align_items(Alignment::Center)
        .into()
    }
}

/// 时间输入面板组件
/// 
/// 提供时间输入的用户界面
#[derive(Debug, Clone)]
pub struct TimeInputPanel {
    /// 时间输入
    pub time_input: TimeInput,
    /// 任务类型
    pub task_type: TaskType,
}

impl Default for TimeInputPanel {
    fn default() -> Self {
        Self {
            time_input: TimeInput::default(),
            task_type: TaskType::Once,
        }
    }
}

impl TimeInputPanel {
    /// 创建新的时间输入面板
    /// 
    /// # 返回值
    /// 
    /// 返回默认的时间输入面板
    pub fn new() -> Self {
        Self::default()
    }
}

/// 倒计时显示组件
/// 
/// 显示当前倒计时状态和剩余时间
#[derive(Debug, Clone)]
pub struct CountdownDisplay {
    /// 是否正在倒计时
    pub is_counting: bool,
    /// 剩余时间（秒）
    pub remaining_seconds: Option<u64>,
    /// 任务类型
    pub task_type: TaskType,
}

impl Default for CountdownDisplay {
    fn default() -> Self {
        Self {
            is_counting: false,
            remaining_seconds: None,
            task_type: TaskType::Once,
        }
    }
}

impl CountdownDisplay {
    /// 创建新的倒计时显示
    /// 
    /// # 返回值
    /// 
    /// 返回默认的倒计时显示
    pub fn new() -> Self {
        Self::default()
    }
}

/// 控制面板组件
/// 
/// 提供开始、取消倒计时等控制功能
#[derive(Debug, Clone)]
pub struct ControlPanel {
    /// 是否正在倒计时
    pub is_counting: bool,
}

impl Default for ControlPanel {
    fn default() -> Self {
        Self {
            is_counting: false,
        }
    }
}

impl ControlPanel {
    /// 创建新的控制面板
    /// 
    /// # 返回值
    /// 
    /// 返回默认的控制面板
    pub fn new() -> Self {
        Self::default()
    }
}