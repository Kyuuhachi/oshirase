use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NotificationData {
	pub app_name: String,
	pub summary: String,
	pub body: String,
	pub actions: Vec<(String, String)>,
	pub expire_timeout: Option<u32>,
	pub urgency: u8,
	pub image: Option<Image>,
	pub extra: HashMap<String, zbus::zvariant::OwnedValue>,
}

#[derive(Debug, Clone)]
pub enum Image {
	Path(String),
	Data(ImageData),
}

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
