mod converter;
mod ui;

use iced::Result;
use ui::App;

fn main() -> Result {
    iced::application(App::new, App::update, App::view)
        .run()
}
