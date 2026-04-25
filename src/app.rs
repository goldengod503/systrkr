use cosmic::app::{Core, Task};
use cosmic::iced::Subscription;
use cosmic::Element;

#[derive(Clone, Debug)]
pub enum Message {
    NoOp,
}

pub struct App {
    core: Core,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "com.system76.SysTrkr";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        (Self { core }, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        cosmic::widget::text("systrkr").into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}
