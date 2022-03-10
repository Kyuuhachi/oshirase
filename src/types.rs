use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NotificationData {
	pub app_name: Option<String>,
	pub title: String,
	pub body: Option<String>,
	pub actions: Vec<(String, String)>,
	pub timeout: Option<std::time::Duration>,
	pub image: Option<Image>,
	pub extra: HashMap<String, zbus::zvariant::OwnedValue>,
}

#[derive(Debug, Clone)]
pub enum Image {
	Path(String),
	Pixbuf(gdk_pixbuf::Pixbuf),
}

pub enum Event {
	Action(String),
	Close(CloseReason),
}

#[derive(Debug, Clone, Copy)]
pub enum CloseReason {
	Expired = 1,
	Dismissed = 2,
	Closed = 3,
	Other = 4,
}

#[derive(Debug, Clone, Copy)]
pub struct Properties {
	pub name: &'static str,
	pub vendor: &'static str,
	pub version: &'static str,
	pub capabilities: &'static [&'static str],
}

pub trait Display {
	const PROPERTIES: Properties;
	fn new(events: glib::Sender<(u32, Event)>) -> Self;
	fn open(&mut self, id: u32, data: NotificationData);
	fn close(&mut self, id: u32);
}
