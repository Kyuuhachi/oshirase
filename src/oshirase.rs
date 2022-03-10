use std::collections::BTreeMap;
use gtk::prelude::*;
use crate::types::*;

pub struct Oshirase {
	events: glib::Sender<(u32, Event)>,
	notifications: BTreeMap<u32, Notification>,
}

struct Notification {
	window: gtk::Window,
}

impl Drop for Notification {
	fn drop(&mut self) {
		unsafe { self.window.destroy() };
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
		let notif = self.notifications.entry(id).or_insert_with(|| new_notification());

		let events = self.events.clone();
		notif.window.add(&make_widget(
			&data,
			move |e| {
				events.send((id, e)).unwrap()
			}
		));
		notif.window.resize(1, 1);
		notif.window.show();
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
		for win in self.notifications.values().filter_map(|n| n.window.window()) {
			if let Some(mon) = win.display().monitor_at_window(&win) {
				let w = mon.geometry().width() * mon.scale_factor();
				win.move_(w - win.width(), y);
				y += win.height();
			}
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

fn new_notification() -> Notification {
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

fn setup_window(window: &gtk::Window) {
	window.connect_draw(|window, _| { window.window().unwrap().set_child_input_shapes(); Inhibit(false) });
}

macro_rules! Box {
	($orient:ident; $($fill:ident: $child:expr),* $(,)?) => { {
		let _b = build!(gtk::Box { orientation: gtk::Orientation::$orient, visible: true });
		$(_b.pack_start($child, $fill, $fill, 0);)*
		_b
	} };
}

fn ebox(child: &impl glib::IsA<gtk::Widget>) -> gtk::EventBox {
	let b = build!(gtk::EventBox { visible: true });
	b.add(child);
	b
}

fn make_widget(
	data: &NotificationData,
	callback: impl Fn(Event) + 'static + Clone,
) -> impl glib::IsA<gtk::Widget> {
	let title = build!(gtk::Label {
		name: "title",
		visible: true,
		xalign: 0.,
		label: &data.title,
	});
	title.set_line_wrap(true);

	let body = build!(gtk::Label {
		name: "body",
		visible: data.body.is_some(),
		xalign: 0.,
		use_markup: true,
	});
	body.set_line_wrap(true);
	if let Some(body_t) = &data.body {
		body.set_markup(body_t);
		body.show();
	}

	let image = match &data.image {
		Some(Image::Pixbuf(pixbuf)) => build!(gtk::Image {
			name: "image",
			visible: true,
			pixbuf: &pixbuf.scale_simple(80, 80, gdk_pixbuf::InterpType::Bilinear).unwrap(),
		}),
		_ => gtk::Image::new(),
	};

	let close = build!(gtk::Button {
		name: "close",
		visible: true,
		halign: gtk::Align::End,
		visible: true,
		relief: gtk::ReliefStyle::None,
		image: &gtk::Image::from_icon_name(Some("window-close"), gtk::IconSize::Button),
	});
	close.connect_clicked(glib::clone!(@strong callback =>
		move |_| callback(Event::Close(CloseReason::Dismissed))
	));

	let actions = build!(gtk::Box {
		name: "actions",
		visible: !data.actions.is_empty(),
		orientation: gtk::Orientation::Vertical,
		valign: gtk::Align::End
	});
	actions.style_context().add_class("linked");
	for (k, v) in &data.actions {
		let v = v.to_owned();
		let btn = build!(gtk::Button {
			label: &k,
			visible: true,
			relief: gtk::ReliefStyle::None,
		});
		btn.connect_clicked(glib::clone!(@strong callback =>
			move |_| callback(Event::Action(v.clone()))
		));
		btn.style_context().add_class("action");
		actions.pack_start(&ebox(&btn), false, false, 0);
	}

	let root = Box!(Horizontal;
		false: &image,
		true: &Box!(Vertical;
			false: &title,
			false: &body,
		),
		false: &Box!(Vertical;
			false: &ebox(&close),
			true: &actions,
		),
	);
	root.set_widget_name("notification");
	root
}
