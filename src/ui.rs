use tinyui::*;


const WIDTH: f64 = 480.;
const HEIGHT: f64 = 160.;

#[derive(Clone, Copy)]
pub struct PluginWindow {
    window: Window,
    button: Button,
    pub counter: Label,
    pub state_label: Label,
}

impl EventHandler for PluginWindow {
    fn handle(&mut self, event: Event) {
        println!("-- event {:?}", event);
        match event {
            Event::ButtonClicked(name) => {
                match name.as_str() {
                    "a button" => { self.button.set_text("clicked me"); }
                    _ => {}
                }
            }
            // Event::WindowWillClose => App::quit(), // don't do this on a vst
            Event::WindowWillClose => {}
            _ => (),
        }
    }
}

impl PluginWindow {
    pub fn new(mut window: Window) -> Self {
        let window_rect = Rect::new(0., 0., WIDTH, HEIGHT);
        let (_top_half_rect, bottom_half_rect) = window_rect.split_horizontal();
        // info!("building window");
        let mut app = Self {
            window: window,
            counter: Label::new("0:00", Rect::new(10., 10., 120., 32.)),
            state_label: Label::new("Stopped", Rect::new(10., 44., 120., 32.)),
            button: ButtonBuilder {
                id: "a button",
                text: "click me",
                style: ButtonStyle::Square,
                position: bottom_half_rect.inset(10.),
            }.build(),
        };

        window.set_title("Echo Looper");
        window.set_background_color(Color::system_gray());
        // app.button.attach(&mut app.window);

        let font = Font::init("Menlo", 24.);
        app.counter.set_font(font);
        app.state_label.set_font(font);

        app.counter.attach(&mut app.window);
        app.state_label.attach(&mut app.window);
        app.window.set_handler(app.clone());

        app
    }

}
