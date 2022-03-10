use std::collections::BTreeMap;
use gtk::prelude::*;
use crate::types::*;

const CSS: &'static str = r#"
#notification {
	background-color: rgba(0,0,0,0.7);
	border-radius: .5em;
	margin-top: .5em;
	margin-left: .5em;
	margin-right: .5em;
	padding: .5em;
	min-width: 30em;
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

pub struct Oshirase {
	events: glib::Sender<(u32, Event)>,
	notifications: BTreeMap<u32, Notification>,
}

struct Notification(gtk::Window, Option<glib::SourceId>);

impl Drop for Notification {
	fn drop(&mut self) {
		unsafe { self.0.destroy() };
		if let Some(a) = self.1.take() { a.remove(); }
	}
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
		self.notifications.remove(&id);

		let events = self.events.clone();
		self.notifications.insert(id, make_notification(&data, move |e| { events.send((id, e)).unwrap() }));
		self.reflow();
	}

	fn close(&mut self, id: u32, _reason: CloseReason) -> bool {
		if self.notifications.remove(&id).is_some() {
			self.reflow();
			true
		} else {
			false
		}
	}
}

impl Oshirase {
	fn reflow(&self) {
		let mut y = 0;
		// Currently does not handle multiple monitors
		for win in self.notifications.values().filter_map(|n| n.0.window()) {
			if let Some(mon) = win.display().monitor_at_window(&win) {
				let w = mon.geometry().width() * mon.scale_factor();
				win.move_(w - win.width(), y);
				y += win.height();
			}
		}
	}
}

macro_rules! build {
	($var:ident @ $name:ty { $($key:ident: $val:expr),* $(,)? }; $($init:tt)*) => {{
		let $var = build!($name { $($key: $val),* });
		{ $($init)* }
		$var
	}};
	($name:ty { $($key:ident: $val:expr),* $(,)? }) => {
		<$name>::builder()
			.visible(true)
			$(.$key($val))*
			.build()
	};
}

fn make_notification(
	data: &NotificationData,
	callback: impl Fn(Event) + 'static + Clone,
) -> Notification {
	let win = build!(gtk::Window {
		visible: false,
		type_hint: gtk::gdk::WindowTypeHint::Notification,
		decorated: false,
		app_paintable: true,
	});
	win.set_visual(win.screen().unwrap().rgba_visual().as_ref());
	win.connect_realize(|win| win.window().unwrap().set_override_redirect(true));
	win.connect_draw(|win, _| { win.window().unwrap().set_child_input_shapes(); Inhibit(false) });

	let urgency: u8 = data.extra.get("urgency").and_then(|a| u8::try_from(a).ok()).unwrap_or(1);
	let timeout = data.timeout.or_else(|| match urgency {
		0 => Some(std::time::Duration::from_secs_f32(3.5)),
		1 => Some(std::time::Duration::from_secs_f32(5.0)),
		_ => None,
	});

	let timeout_source = timeout.map(|t| glib::timeout_add_local(t,
		glib::clone!(@strong callback => move || {
			callback(Event::Close(CloseReason::Expired));
			Continue(true)
			// It'll be removed when the notif is dropped.
			// Removing it here causes a panic because it's elready gone when it's dropped.
		})
	));

	win.add(&make_widget(&data, callback));

	win.resize(1, 1);
	win.show();
	Notification(win, timeout_source)
}

fn ebox(child: &impl glib::IsA<gtk::Widget>) -> gtk::EventBox {
	build!(a@gtk::EventBox {}; a.add(child))
}

fn make_widget(
	data: &NotificationData,
	callback: impl Fn(Event) + 'static + Clone,
) -> impl glib::IsA<gtk::Widget> {
	let title = build!(
		a@gtk::Label {
			name: "title",
			xalign: 0.,
			label: &data.title,
		};
		a.set_line_wrap(true);
	);

	let body = build!(
		a@gtk::Label {
			name: "body",
			visible: data.body.is_some(),
			xalign: 0.,
			use_markup: true,
		};
		a.set_line_wrap(true);
		if let Some(body_t) = &data.body {
			a.set_markup(body_t);
			a.show();
		}
	);

	let image = match &data.image {
		Some(Image::Pixbuf(pixbuf)) => build!(gtk::Image {
			name: "image",
			pixbuf: &pixbuf.scale_simple(80, 80, gdk_pixbuf::InterpType::Bilinear).unwrap(),
		}),
		_ => gtk::Image::new(),
	};

	let close = build!(
		a@gtk::Button {
			name: "close",
			halign: gtk::Align::End,
			relief: gtk::ReliefStyle::None,
			image: &gtk::Image::from_icon_name(Some("window-close"), gtk::IconSize::Button),
		};
		a.connect_clicked(glib::clone!(@strong callback =>
			move |_| callback(Event::Close(CloseReason::Dismissed))
		));
	);

	let actions = build!(
		a@gtk::Box {
			name: "actions",
			visible: !data.actions.is_empty(),
			orientation: gtk::Orientation::Vertical,
			halign: gtk::Align::End,
			valign: gtk::Align::End,
		};
		a.style_context().add_class("linked");
	);
	
	data.actions.iter().map(|(k, v)| build!(
		a@gtk::Button {
			label: &k,
			relief: gtk::ReliefStyle::None,
		};
		a.connect_clicked(glib::clone!(@strong callback, @strong v =>
			move |_| callback(Event::Action(v.clone()))
		));
		a.style_context().add_class("action");
	)).for_each(|a| actions.pack_start(&ebox(&a), false, false, 0));

	build!(
		a@gtk::Box { name: "notification", orientation: gtk::Orientation::Horizontal };
		a.pack_start(&image, false, false, 0);
		a.pack_start(&build!(
			a@gtk::Box { orientation: gtk::Orientation::Vertical };
			a.pack_start(&title, false, false, 0);
			a.pack_start(&body, false, false, 0);
		), true, true, 0);
		a.pack_start(&build!(
			a@gtk::Box { orientation: gtk::Orientation::Vertical };
			a.pack_start(&ebox(&close), false, false, 0);
			a.pack_end(&actions, false, false, 0);
		), false, false, 0);
	)
}
