mod server;
mod oshirase;
mod types;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	server::main::<oshirase::Oshirase>().await
}
