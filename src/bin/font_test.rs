//! 字体测试程序
//! 
//! 用于测试中文字体显示是否正常

use iced::{
    widget::{button, column, container, text},
    Application, Command, Element, Length, Settings, Theme, Font,
};

/// 字体测试应用
#[derive(Debug, Default)]
struct FontTestApp {
    counter: i32,
}

/// 消息类型
#[derive(Debug, Clone)]
enum Message {
    Increment,
    Decrement,
}

impl Application for FontTestApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        "中文字体测试 - QtShut".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Increment => {
                self.counter += 1;
            }
            Message::Decrement => {
                self.counter -= 1;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let content = column![
            text("QtShut - 定时关机工具").size(24),
            text(format!("计数器: {}", self.counter)).size(18),
            text("这是中文字体测试").size(16),
            text("按钮测试:").size(14),
            button("增加计数").on_press(Message::Increment),
            button("减少计数").on_press(Message::Decrement),
            text("时间设置: 30分钟").size(14),
            text("状态: 倒计时进行中").size(14),
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

fn main() -> iced::Result {
    let settings = Settings {
        default_font: Font::with_name("Microsoft YaHei"),
        window: iced::window::Settings {
            size: iced::Size::new(400.0, 300.0),
            ..Default::default()
        },
        ..Settings::default()
    };
    
    FontTestApp::run(settings)
}