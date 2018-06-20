use tinyui::*;


const WIDTH: f64 = 480.;
const HEIGHT: f64 = 160.;

#[derive(Clone, Copy)]
pub struct PluginWindow {
    window: Window,
   // button: Button,
    pub counter: Label,
    pub state_label: Label,
    pub cycle_label: Label,
    pub version_label: Label,
}

impl EventHandler for PluginWindow {
    fn handle(&mut self, event: Event) {
        println!("-- event {:?}", event);
        match event {
            Event::ButtonClicked(name) => {
                match name.as_str() {
                   //  "a button" => { self.button.set_text("clicked me"); }
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
        let (_top_half_rect, _bottom_half_rect) = window_rect.split_horizontal();
        // info!("building window");
        let mut app = Self {
            window: window,
            counter: Label::new("0:00", Rect::new(10., 10., 160., 40. )),
            cycle_label: Label::new("1 | 1", Rect::new(160., 10., 80., 40. )),
            state_label: Label::new("Stopped", Rect::new(10., 50., 120., 40. )),
            version_label: Label::new("PlexLooper v0000", Rect::new(380., 10., 120., 16.)),
//            button: ButtonBuilder {
//                id: "a button",
//                text: "click me",
//                style: ButtonStyle::Square,
//                position: bottom_half_rect.inset(10.),
//            }.build(),
        };

        window.set_title("Plex Looper");
        window.set_background_color(Color::system_gray());
        // app.button.attach(&mut app.window);

        let font = Font::init("Menlo", 24.);
        let version_font = Font::system_font(0.);
        app.counter.set_font(font);
        app.cycle_label.set_font(font);
        app.state_label.set_font(font);
        app.version_label.set_font(version_font);

        app.counter.attach(&mut app.window);
        app.cycle_label.attach(&mut app.window);
        app.state_label.attach(&mut app.window);
        app.version_label.attach(&mut app.window);
        app.version_label.set_text("Version v0.0.1.1");
        app.window.set_handler(app.clone());

        app
    }

}
