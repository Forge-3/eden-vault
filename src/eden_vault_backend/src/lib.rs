mod state;
mod types;

#[ic_cdk::query]
fn elo(name: String) -> String {
    format!("Hello, {}!", name)
}
