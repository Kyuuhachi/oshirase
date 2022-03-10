use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NotificationData {
	pub app_name: Option<String>,
	pub summary: String,
	pub body: Option<String>,
	pub actions: Vec<(String, String)>,
	pub expire_timeout: Option<u32>,
	pub urgency: u8,
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
