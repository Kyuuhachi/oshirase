use crate::types::*;

pub struct Oshirase {
	events: glib::Sender<(u32, Event)>,
}

impl Display for Oshirase {
	const PROPERTIES: Properties = Properties {
		name: "Oshirase",
		vendor: "Kyuuhachi",
		version: "0.1",
		capabilities: &["actions", "body", "body-markup", "icon-static"],
	};

	fn new(events: glib::Sender<(u32, Event)>) -> Oshirase {
		Oshirase {
			events,
		}
	}

	fn open(&mut self, id: u32, data: NotificationData) {
		println!("open({:?}, {:?}", id, data);
	}

	fn close(&mut self, id: u32) {
		self.events.send((id, Event::Close(CloseReason::Closed))).unwrap();
	}
}
