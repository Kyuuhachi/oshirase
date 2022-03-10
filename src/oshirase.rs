use crate::notification::*;

pub struct Oshirase {
	events: glib::Sender<(u32, Event)>,
}

impl Oshirase {
	pub const NAME: &'static str = "Oshirase";
	pub const VENDOR: &'static str = "Kyuuhachi";
	pub const VERSION: &'static str = "0.1";
	pub const CAPABILITIES: &'static [&'static str] = &["actions", "body", "body-markup", "icon-static"];

	pub fn new(events: glib::Sender<(u32, Event)>) -> Self {
		Self {
			events,
		}
	}

	pub fn open(&mut self, id: u32, data: NotificationData) {
		println!("open({:?}, {:?}", id, data);
	}

	pub fn close(&mut self, id: u32) {
		self.events.send((id, Event::Close(CloseReason::Closed))).unwrap();
	}
}
