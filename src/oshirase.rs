use std::{collections::BTreeMap, rc::Rc, cell::RefCell};
use gtk::prelude::*;
use crate::types::*;

type Notifications = Rc<RefCell<BTreeMap<u32, Notification>>>;

pub struct Oshirase {
	events: glib::Sender<(u32, Event)>,
	notifications: Notifications,
}

struct Notification {
	window: gtk::Window,
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
			notifications: Default::default(),
		}
	}

	fn open(&mut self, id: u32, data: NotificationData) {
		println!("open({:?}, {:?}", id, data);

		let mut notifications = self.notifications.borrow_mut();
		notifications.entry(id).or_insert_with(|| new_notification(&self.notifications));
		drop(notifications);

		let notifications = self.notifications.borrow();
		let notif = notifications.get(&id).unwrap();
		notif.window.show();
	}

	fn close(&mut self, id: u32) {
		if let Some(_notif) = self.notifications.borrow_mut().remove(&id) {
			self.events.send((id, Event::Close(CloseReason::Closed))).unwrap();
		}
	}
}

fn reflow(notifs: &Notifications) {
	let mut y = 0;
	// Currently does not handle multiple monitors
	for win in notifs.borrow().values().filter_map(|n| n.window.window()) {
		if let Some(mon) = win.display().monitor_at_window(&win) {
			let w = mon.geometry().width() * mon.scale_factor();
			win.move_(w - win.width(), y);
			y += win.height();
		}
	}
}

fn new_notification(notifs: &Notifications) -> Notification {
	let window = gtk::Window::builder()
		.type_hint(gtk::gdk::WindowTypeHint::Notification)
		.decorated(false)
		// .app_paintable(true)
		.build();

	fn set_rgba_visual(window: &gtk::Window) {
		println!("{:?}", window.screen());
		if let Some(screen) = window.screen() {
			window.set_visual(screen.rgba_visual().as_ref());
		}
	}

	window.connect_screen_changed(|win, _| set_rgba_visual(win));
	set_rgba_visual(&window);
	window.connect_realize(|win| win.window().unwrap().set_override_redirect(true));

	window.connect_show(glib::clone!(@weak notifs => move |_| reflow(&notifs)));
	window.connect_hide(glib::clone!(@weak notifs => move |_| reflow(&notifs)));

	Notification {
		window
	}
}
