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
		let css = gtk::CssProvider::new();
		css.load_from_data(CSS.as_bytes()).expect("Failed to load css");
		gtk::StyleContext::add_provider_for_screen(
			&gtk::gdk::Screen::default().unwrap(),
			&css,
			gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
		);

		Oshirase {
			events,
			notifications: Default::default(),
		}
	}

	fn open(&mut self, id: u32, data: NotificationData) {
		let mut notifications = self.notifications.borrow_mut();
		notifications.entry(id).or_insert_with(|| new_notification(&self.notifications));
		drop(notifications);

		let notifications = self.notifications.borrow();
		let notif = notifications.get(&id).unwrap();
		notif.window.add(&make_widget(&data));
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

macro_rules! build {
	($name:ty { $($key:ident: $val:expr),* $(,)? }) => {{
		let v = <$name>::builder();
		$(let v = v.$key($val);)*
		v.build()
	}};
}

fn new_notification(notifs: &Notifications) -> Notification {
	let window = build!(gtk::Window {
		type_hint: gtk::gdk::WindowTypeHint::Notification,
		decorated: false,
		app_paintable: true,
	});

	fn set_rgba_visual(window: &gtk::Window) {
		if let Some(screen) = window.screen() {
			window.set_visual(screen.rgba_visual().as_ref());
		}
	}
	window.connect_screen_changed(|win, _| set_rgba_visual(win));
	set_rgba_visual(&window);

	window.connect_realize(|win| win.window().unwrap().set_override_redirect(true));

	window.connect_show(glib::clone!(@weak notifs => move |_| reflow(&notifs)));
	window.connect_hide(glib::clone!(@weak notifs => move |_| reflow(&notifs)));

	setup_window(&window);

	Notification {
		window
	}
}

const CSS: &'static str = r#"
#notification {
	background-color: rgba(0,0,0,0.7);
	border-radius: .5em;
	margin-top: .5em;
	margin-left: .5em;
	margin-right: .5em;
	padding: .5em;
	min-width: 20em;
}

#title {
	font-weight: bold;
	font-size: 125%;
}

#image {
	padding-right: .5em;
}

#actions {
	border-left: 1px solid rgba(255,255,255,0.3);
	padding-left: .5em;
}

button {
	padding: 0;
	min-width: 0;
	background: none;
	border: none;
}

button:not(:hover) {
	opacity: 0.4;
	border: none;
}
"#;

fn setup_window(window: &gtk::Window) {
	window.connect_draw(|window, _| { window.window().unwrap().set_child_input_shapes(); Inhibit(false) });
}

fn make_widget(data: &NotificationData) -> impl glib::IsA<gtk::Widget> {
	let title = build!(gtk::Label {
		name: "title",
		visible: true,
		xalign: 0.,
		label: &data.title,
	});
	title.set_line_wrap(true);

	let body = build!(gtk::Label {
		name: "body",
		visible: true,
		xalign: 0.,
		use_markup: true,
	});
	body.set_line_wrap(true);
	if let Some(body_t) = &data.body {
		body.set_markup(body_t);
		body.show();
	}

	let close = build!(gtk::Button {
		name: "close",
		visible: true,
		halign: gtk::Align::End,
		visible: true,
		relief: gtk::ReliefStyle::None,
		image: &gtk::Image::from_icon_name(Some("window-close"), gtk::IconSize::Button),
	});
	let close = { let b = build!(gtk::EventBox { visible: true }); b.add(&close); b };

	let actions = build!(gtk::Box {
		name: "actions",
		visible: true,
		orientation: gtk::Orientation::Vertical,
		valign: gtk::Align::End
	});
	actions.style_context().add_class("linked");

	macro_rules! Box {
		($orient:ident; $($fill:ident: $child:expr),* $(,)?) => { {
			let _b = build!(gtk::Box { orientation: gtk::Orientation::$orient, visible: true });
			$(_b.pack_start(&$child, $fill, $fill, 0);)*
			_b
		} };
	}

	let root = Box!(Horizontal;
		// false: image,
		true: Box!(Vertical;
			false: title,
			false: body,
		),
		false: Box!(Vertical;
			false: close,
			true: actions,
		),
	);
	root.set_widget_name("notification");
	root
}
