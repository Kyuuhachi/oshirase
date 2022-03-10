use std::collections::HashMap;

mod oshirase;
mod notification;

use notification::*;

#[derive(Debug)]
enum Message {
	Open(u32, OpenMessage),
	Close(u32),
}

#[derive(Debug)]
struct OpenMessage {
	app_name: String,
	app_icon: String,
	summary: String,
	body: String,
	actions: Vec<String>,
	hints: HashMap<String, zbus::zvariant::OwnedValue>,
	expire_timeout: i32,
}

#[derive(Debug)]
struct OshiraseServer(u32, glib::Sender<Message>);

#[zbus::dbus_interface(name = "org.freedesktop.Notifications")]
impl OshiraseServer {
	async fn get_server_information(&self) -> (&str, &str, &str, &str) {
		("Oshirase", "Kyuuhachi", "0.1", "1.1")
	}

	async fn get_capabilities(&self) -> &[&str] {
		&["actions", "body", "body-markup", "icon-static"]
	}

	// This is type (susssasa{sv}i) rather than susssasa{sv}i, but it seems to work.
	async fn notify(
		&mut self,
		app_name: String,
		replaces_id: u32,
		app_icon: String,
		summary: String,
		body: String,
		actions: Vec<String>,
		hints: HashMap<String, zbus::zvariant::OwnedValue>,
		expire_timeout: i32,
	) -> u32 {
		let id = if replaces_id == 0 { self.0 += 1; self.0 } else { replaces_id };
		self.1.send(Message::Open(id, OpenMessage {
			app_name,
			app_icon,
			summary,
			body,
			actions,
			hints,
			expire_timeout,
		})).unwrap();
		id
	}

	async fn close_notification(&self, id: u32) {
		self.1.send(Message::Close(id)).unwrap();
	}

	#[dbus_interface(signal)]
	async fn notification_closed(&self, ctx: &zbus::SignalContext<'_>, id: u32, reason: u32) -> zbus::Result<()>;

	#[dbus_interface(signal)]
	async fn action_invoked(&self, ctxt: &zbus::SignalContext<'_>, id: u32, action: &str) -> zbus::Result<()>;
}

fn image_data(value: zbus::zvariant::OwnedValue) -> Option<Image> {
	#[derive(Debug, zbus::zvariant::Value, Clone, zbus::zvariant::OwnedValue)]
	pub struct ImageData {
		pub width: i32,
		pub height: i32,
		pub rowstride: i32,
		pub has_alpha: bool,
		pub bits_per_sample: i32,
		pub channels: i32,
		pub data: Vec<u8>,
	}

	let data = ImageData::try_from(value).ok()?;
	Some(Image::Pixbuf(gdk_pixbuf::Pixbuf::from_bytes(
		&glib::Bytes::from_owned(data.data),
		gdk_pixbuf::Colorspace::Rgb,
		data.has_alpha,
		data.bits_per_sample,
		data.width,
		data.height,
		data.rowstride
	)))
}

fn parse_data(msg: OpenMessage) -> NotificationData {
	let mut hints = msg.hints;
	let app_icon = if !msg.app_icon.is_empty() { Some(msg.app_icon) } else { None };
	let expire_timeout = u32::try_from(msg.expire_timeout).ok();
	let actions = msg.actions
		.chunks_exact(2)
		.map(|a| if let [a, b] = a { (a.clone(), b.clone()) } else { unreachable!() })
		.collect::<Vec<_>>();

	let urgency: u8 = hints.remove("urgency").and_then(|a| u8::try_from(a).ok()).unwrap_or(1);

	// Slightly inefficient if multiple exist, but I want to remove them all from the map
	let image = None
		.or(hints.remove("image-data").and_then(|a| image_data(a)))
		.or(hints.remove("image_data").and_then(|a| image_data(a)))
		.or(hints.remove("image-path").and_then(|a| String::try_from(a).ok()).map(|a| Image::Path(a)))
		.or(hints.remove("image_path").and_then(|a| String::try_from(a).ok()).map(|a| Image::Path(a)))
		.or(app_icon                  .and_then(|a| String::try_from(a).ok()).map(|a| Image::Path(a)))
		.or(hints.remove("icon_data") .and_then(|a| image_data(a)))
	;

	NotificationData {
		app_name: msg.app_name,
		summary: msg.summary,
		body: msg.body,
		actions,
		expire_timeout,
		urgency,
		image,
		extra: hints
	}
}


#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let main_context = glib::MainContext::default();
	let _context = main_context.acquire()?;
	gtk::init()?;

	let (dbus_tx, dbus_rx) = glib::MainContext::channel::<Message>(glib::PRIORITY_DEFAULT);
	let (action_tx, action_rx) = glib::MainContext::channel::<(u32, Event)>(glib::PRIORITY_DEFAULT);

	let conn = zbus::ConnectionBuilder::session()?
		.name("org.freedesktop.Notifications")?
		.serve_at("/org/freedesktop/Notifications", OshiraseServer(0, dbus_tx))?
		.build().await?;

	action_rx.attach(Some(&main_context), glib::clone!(@strong conn => move |(id, event)| {
		let conn = conn.clone();
		gidle_future::spawn(async move {
			let server_ref = conn
				.object_server()
				.interface::<_, OshiraseServer>("/org/freedesktop/Notifications").await.unwrap();
			let server = server_ref.get().await;
			let ctx = server_ref.signal_context();
			match event {
				Event::Action(a) => server.action_invoked(ctx, id, &a).await.unwrap(),
				Event::Close(r) => server.notification_closed(ctx, id, r as u32).await.unwrap(),
			}
		});
		glib::Continue(true)
	}));

	let mut oshirase = oshirase::Oshirase::new(action_tx);

	dbus_rx.attach(Some(&main_context), move |msg| {
		match msg {
			Message::Open(id, msg) => {
				oshirase.open(id, parse_data(msg));
			}
			Message::Close(id) => {
				oshirase.close(id);
			}
		}
		glib::Continue(true)
	});

	gtk::main();
	Ok(())
}
