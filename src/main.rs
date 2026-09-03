use winpie::app::Application;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Application::new()?;
    app.run()
}
