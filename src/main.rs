use std::collections::HashMap;

use gtk::prelude::*;

#[derive(Debug)]
enum Message {
	Open(u32, NotifyData),
	Close(u32),
}

#[derive(Debug)]
struct NotifyData {
	app_name: String,
	summary: String,
	body: String,
	actions: Vec<String>,
	expire_timeout: Option<u32>,

	urgency: u8,
	image: Option<Image>,
	extra: HashMap<String, zbus::zvariant::OwnedValue>,
}

#[derive(Debug)]
enum Image {
	Path(String),
	Data(ImageData),
}


#[derive(Debug, zbus::zvariant::Value, zbus::zvariant::OwnedValue)]
struct ImageData {
	width: i32,
	height: i32,
	rowstride: i32,
	has_alpha: bool,
	bits_per_sample: i32,
	channels: i32,
	data: Vec<u8>,
}

#[derive(serde::Serialize, Debug, Clone, Copy, zbus::zvariant::Type)]
enum CloseReason {
	Expired = 1,
	Dismissed = 2,
	Closed = 3,
	Other = 4,
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

		let mut hints = hints;
		let app_icon = if !app_icon.is_empty() { Some(app_icon) } else { None };
		let expire_timeout = u32::try_from(expire_timeout).ok();

		let urgency: u8 = hints.remove("urgency").and_then(|a| u8::try_from(a).ok()).unwrap_or(1);

		// Slightly inefficient if multiple exist, but I want to remove them all from the map
		let image = None
			.or(hints.remove("image-data").and_then(|a| ImageData::try_from(a).ok()).map(|a| Image::Data(a)))
			.or(hints.remove("image_data").and_then(|a| ImageData::try_from(a).ok()).map(|a| Image::Data(a)))
			.or(hints.remove("image-path").and_then(|a| String   ::try_from(a).ok()).map(|a| Image::Path(a)))
			.or(hints.remove("image_path").and_then(|a| String   ::try_from(a).ok()).map(|a| Image::Path(a)))
			.or(app_icon                  .and_then(|a| String   ::try_from(a).ok()).map(|a| Image::Path(a)))
			.or(hints.remove("icon_data") .and_then(|a| ImageData::try_from(a).ok()).map(|a| Image::Data(a)))
		;

		let data = NotifyData {
			app_name,
			summary,
			body,
			actions,
			expire_timeout,

			urgency,
			image,
			extra: hints
		};
		self.1.send(Message::Open(id, data)).unwrap();
		id
	}

	async fn close_notification(&self, id: u32) {
		self.1.send(Message::Close(id)).unwrap();
	}

	#[dbus_interface(signal)]
	async fn notification_closed(&self, ctx: &zbus::SignalContext<'_>, id: u32, reason: CloseReason) -> zbus::Result<()>;

	#[dbus_interface(signal)]
	async fn action_invoked(&self, ctxt: &zbus::SignalContext<'_>, id: u32, action: &str) -> zbus::Result<()>;
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let main_context = glib::MainContext::default();
	let _context = main_context.acquire()?;
	let (tx, rx) = glib::MainContext::channel::<Message>(glib::PRIORITY_DEFAULT);
	gtk::init()?;

	let server = zbus::ConnectionBuilder::session()?
		.name("org.freedesktop.Notifications")?
		.serve_at("/org/freedesktop/Notifications", OshiraseServer(0, tx))?
		.build().await?
		.object_server()
		.interface::<_, OshiraseServer>("/org/freedesktop/Notifications").await?;

	rx.attach(Some(&main_context), move |msg| {
		println!("{:?}", msg);
		glib::Continue(true)
	});

	gtk::main();
	Ok(())
}
