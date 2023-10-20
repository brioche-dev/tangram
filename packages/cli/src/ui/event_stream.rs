use crossterm as ct;
use tangram_client as tg;

pub enum Event {
    Terminal(ct::event::Event),
    Child(ChildEvent),
    Completed(tg::Result<()>),
}

pub struct ChildEvent {
    pub parent: tg::build::Id,
    pub child: tg::Result<tg::Build>,
}

struct EventQueue {
    channel: tokio::sync::mpsc::Receiver<Event>,
}

impl Drop for EventQueue {
    fn drop(&mut self) {
        self.channel.close();
    }
}

impl EventQueue {
    pub fn next(&mut self) -> Option<Event> {
        self.channel.blocking_recv()
    }
}
