use everty_ui::{IconId, Rgba8Renderer};
pub fn register(renderer: &mut Rgba8Renderer) {
    for (id,path) in [
        (1,"M4 16l-1 5 5-1L20 8l-4-4L4 16z M14 6l4 4"),
        (2,"M3 6h18 M9 6V3h6v3 M5 6l1 15h12l1-15 M10 10v7 M14 10v7"),
        (3,"M20 8a8 8 0 1 0 0 9 M20 3v5h-5"),
        (4,"M10 14l4-4 M8 16l-2 2a4 4 0 0 1-6-6l5-5a4 4 0 0 1 6 0 M16 8l2-2a4 4 0 0 1 6 6l-5 5a4 4 0 0 1-6 0"),
        (5,"M12 4v16 M4 12h16"),
        (6,"M12 3v12 M7 8l5-5 5 5 M4 15v6h16v-6"),
        (7,"M12 3v12 M7 10l5 5 5-5 M4 16v5h16v-5"),
        (8,"M5 12l4 4L19 6"),
        (9,"M12 2v10 M6 5a9 9 0 1 0 12 0"),
        (10,"M4 6h16 M4 12h11 M4 18h6 M18 12v9 M15 18l3 3 3-3"),
        (11,"M4 4h16v16H4z M8 16h8V8H8z"),
        (12,"M3 5h18v14H3z M8 22h8 M12 19v3"),
        (13,"M12 8v-1 M12 11v6 M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20"),
        (14,"M3 7h18 M17 3l4 4-4 4 M21 17H3 M7 13l-4 4 4 4"),
        (15,"M4 5h16 M4 12h16 M4 19h16 M8 2v6 M16 9v6 M10 16v6"),
        (16,"M4 3h16v7H4z M4 14h16v7H4z M7 6h1 M7 17h1"),
        (17,"M3 10l9-7 9 7 M5 9v12h14V9 M9 21v-8h6v8"),
        (18,"M9 14a6 6 0 1 0-6-6a6 6 0 0 0 6 6z M13 12l8 8 M17 16l3-3"),
        (19,"M4 3v18h17 M8 16v-4 M13 16V8 M18 16V4"),
        (20,"M4 3h16v18H4z M8 8h8 M8 12h8 M8 16h5"),
    ] {
        let svg=format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="-1 -1 26 26"><path d="{path}" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#);
        renderer.register_svg_icon(IconId(id),&svg).expect("Application SVG icon");
    }
}
