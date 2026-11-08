use liquid_canvas_integration::{
    build_app, command, liquid_snapshot, Fixture, EDIT, HEIGHT, WIDTH,
};
use rndr_core::{KeyEvent, LogicalSize, Modifiers, NamedKey, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut app = build_app();
    for _ in 0..3 {
        rndr_paint::frame(&mut app);
    }
    if let Some(stage) = args
        .iter()
        .position(|a| a == "--stage")
        .and_then(|i| args.get(i + 1))
    {
        let view = app.world().resource::<Fixture>().unwrap().liquid;
        let result = rndr_liquid_ui::update_liquid_source(
            app.world_mut(),
            view,
            &liquid_snapshot(stage.parse()?),
        );
        println!("Snapshot: {result:?}");
    }
    if args.iter().any(|a| a == "--editing") {
        command(app.world_mut(), EDIT);
        rndr_paint::frame(&mut app);
        app.world_mut()
            .resource_mut::<rndr_core::Input>()
            .unwrap()
            .keys = vec![
            KeyEvent::named(NamedKey::Home, Modifiers::new()),
            KeyEvent::named(
                NamedKey::ArrowRight,
                Modifiers {
                    shift: true,
                    ctrl: true,
                    ..Modifiers::new()
                },
            ),
        ];
    }
    if let Some(path) = args
        .iter()
        .position(|a| a == "--export")
        .and_then(|i| args.get(i + 1))
    {
        for _ in 0..3 {
            rndr_paint::frame(&mut app);
        }
        let frame = rndr_paint::frame(&mut app);
        let image = rndr_render::Exporter::new()?.render_frame(
            Some(app.world()),
            &frame,
            WIDTH as u32,
            HEIGHT as u32,
            1.0,
        )?;
        std::fs::write(path, rndr_render::encode_png(&image)?)?;
        println!("Exported {path} ({}x{})", image.width, image.height);
        return Ok(());
    }
    app.world_mut().insert_resource(WindowConfig {
        title: "Liquid + Canvas integration".into(),
        size: LogicalSize::new(WIDTH, HEIGHT),
        ..WindowConfig::default()
    });
    rndr_render::run(app);
    Ok(())
}
