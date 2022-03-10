use crate::notification::*;

pub struct Oshirase {
	events: glib::Sender<(u32, Event)>,
}

impl Oshirase {
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
