use bevy::{
    asset::RenderAssetUsages,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use chrono::Utc;

const BRUSH_THICKNESS: u32 = 3;
const BRUSH_COLOR: Color = Color::linear_rgb(255.0, 255.0, 255.0);
const BOARD_COLOR: Color = Color::linear_rgb(0.0, 0.0, 0.0);
const N_RESAMPLED_POINTS: usize = 64;

#[derive(Resource)]
struct DrawingBoard(Handle<Image>);

#[derive(Component)]
struct ResultText;

#[derive(Resource)]
struct BrushEnabled(bool);

#[derive(Component)]
struct ToggleBrushButton;

#[derive(PartialEq)]
enum DrawMoment {
    Idle,
    Ended,
    Paused,
    Began(Vec2, bool), // the bool is to check if it previously it was paused or not
    Drawing(Vec2),
}

#[derive(Resource)]
struct DrawState(DrawMoment);

fn resample(candidate_vectors: &Vec<Vec<Vec2>>, total_length: f32) -> Vec<Vec2> {
    let mut resampled_points: Vec<Vec2> = Vec::with_capacity(N_RESAMPLED_POINTS);
    let increment = total_length / N_RESAMPLED_POINTS as f32;

    for candidate_points in candidate_vectors.iter() {
        if candidate_points.len() > 1 {
            resampled_points.push(candidate_points[0]);

            let mut accumulated_distance = 0.0;
            let mut previous_point = candidate_points[0];

            for i in 1..candidate_points.len() {
                let current_point = candidate_points[i];
                let mut segment_distance = previous_point.distance(current_point);

                while segment_distance + accumulated_distance >= increment
                    && resampled_points.len() < N_RESAMPLED_POINTS
                {
                    let alpha = (increment - accumulated_distance) / segment_distance;
                    let dv = previous_point.lerp(current_point, alpha);

                    resampled_points.push(dv);

                    previous_point = dv;
                    accumulated_distance = 0.0;
                    segment_distance = dv.distance(current_point);
                }

                accumulated_distance += segment_distance;
                previous_point = current_point;
            }
        }
    }

    resampled_points
}

fn get_centroid(points: &Vec<Vec2>) -> Vec2 {
    let mut c_x = 0.0;
    let mut c_y = 0.0;
    for point in points.iter() {
        c_x += point.x;
        c_y += point.y;
    }
    c_x /= points.len() as f32;
    c_y /= points.len() as f32;
    Vec2::new(c_x, c_y)
}

fn scale_and_translate(points: &mut Vec<Vec2>) {
    // bounding box
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for point in points.iter() {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    // scale
    let scale = f32::max(max_x - min_x, max_y - min_y);
    for point in points.iter_mut() {
        point.x = (point.x - min_x) / scale;
        point.y = (point.y - min_y) / scale;
    }

    // translate to origin
    let centroid = get_centroid(&points);
    for point in points.iter_mut() {
        point.x -= centroid.x;
        point.y -= centroid.y;
    }
}

fn reset_board(window_size: Vec2, board: &mut Image, resize: bool) {
    if resize {
        board.resize(Extent3d {
            width: window_size.x as u32,
            height: window_size.y as u32,
            depth_or_array_layers: 1,
        });
    }

    for x in 0..(window_size.x as u32) {
        for y in 0..(window_size.y as u32) {
            board.set_color_at(x, y, BOARD_COLOR).unwrap_or(());
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    text_color: Color::linear_rgb(0.0, 255.0, 0.0),
                    enabled: true,
                },
            },
        ))
        .add_systems(Startup, (setup_window, spawn))
        .add_systems(Update, (toggle_brush, draw_state_handler, draw).chain())
        .insert_resource(BrushEnabled(true))
        .insert_resource(DrawState(DrawMoment::Idle))
        .run();
}

fn toggle_brush(
    mut brush_enabled: ResMut<BrushEnabled>,
    mut interaction_query: Query<
        (&Interaction, &mut BorderColor),
        (Changed<Interaction>, With<ToggleBrushButton>),
    >,
    mut text: Single<&mut Text, With<ToggleBrushButton>>,
) {
    for (interaction, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                brush_enabled.0 = !brush_enabled.0;
                border_color.0 = bevy::color::palettes::css::LIGHT_GREEN.into();
                text.0 = if brush_enabled.0 {
                    format!("ON")
                } else {
                    format!("OFF")
                };
            }
            _ => {
                text.0 = format!("Toggle Brush");
                border_color.0 = Color::WHITE;
            }
        }
    }
}

fn draw_state_handler(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mouse_move_delta: Res<AccumulatedMouseMotion>,
    mut draw_state: ResMut<DrawState>,
    window: Single<&Window>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        if let Some(x) = window.cursor_position() {
            draw_state.0 = DrawMoment::Began(x, draw_state.0 == DrawMoment::Paused);
        }
    } else if buttons.pressed(MouseButton::Left) && mouse_move_delta.delta != Vec2::ZERO {
        if let Some(x) = window.cursor_position() {
            draw_state.0 = DrawMoment::Drawing(x);
        }
    } else {
        if draw_state.0 != DrawMoment::Paused {
            draw_state.0 = DrawMoment::Idle;
        }

        for touch in touches.iter() {
            if touches.just_pressed(touch.id()) {
                draw_state.0 =
                    DrawMoment::Began(touch.position(), draw_state.0 == DrawMoment::Paused);
            } else if touch.delta() != Vec2::ZERO {
                draw_state.0 = DrawMoment::Drawing(touch.position());
            }
            break;
        }
    }

    if buttons.just_released(MouseButton::Left) || touches.any_just_released() {
        draw_state.0 = DrawMoment::Paused;
    }

    // implement button for phone
    if buttons.just_released(MouseButton::Right) {
        draw_state.0 = DrawMoment::Ended
    }
}

fn fill_pixel(board: &mut Image, vec: Vec2, first_pixel: bool, brush_enabled: bool) {
    let thickness = if first_pixel {
        BRUSH_THICKNESS * 2
    } else {
        BRUSH_THICKNESS
    };
    if brush_enabled {
        for theta in 0..=360 {
            for delta_r in 0..=thickness {
                let x = vec.x + (delta_r as f32) * ops::cos((theta as f32).to_radians());
                let y = vec.y + (delta_r as f32) * ops::sin((theta as f32).to_radians());
                board
                    .set_color_at(x as u32, y as u32, BRUSH_COLOR)
                    .unwrap_or(()); // most likely the error would be an out_of_bounds so it i think im okay to ignore
            }
        }
    } else {
        board
            .set_color_at(vec.x as u32, vec.y as u32, BRUSH_COLOR)
            .unwrap_or(()); // most likely the error would be an out_of_bounds so it i think im okay to ignore
    }
}

fn draw(
    mut result_text: Single<&mut Text, With<ResultText>>,
    drawingboard: Res<DrawingBoard>,
    mut images: ResMut<Assets<Image>>,

    window: Single<&Window>,

    mut previous_pos: Local<Vec2>,
    mut stroke_index: Local<usize>,
    mut candidate_vectors: Local<Vec<Vec<Vec2>>>,
    mut total_length: Local<f32>,

    mut draw_state: ResMut<DrawState>,
    brush_enabled: Res<BrushEnabled>,
) {
    if let DrawMoment::Began(mouse_pos, paused) = draw_state.0 {
        result_text.0 = "".to_string();
        let board = images.get_mut(&drawingboard.0).expect("Board not found!!");

        if !paused {
            candidate_vectors.clear();
            candidate_vectors.push(vec![]);
            *total_length = 0.0;
            reset_board(window.size(), board, true);
        } else {
            *stroke_index += 1;
            candidate_vectors.push(vec![]);
        }

        fill_pixel(board, mouse_pos, true, brush_enabled.0);
        *previous_pos = mouse_pos;
        candidate_vectors[*stroke_index].push(mouse_pos);
    } else if draw_state.0 == DrawMoment::Ended {
        let start_time = Utc::now();

        let mut resampled_points = resample(&candidate_vectors, *total_length);
        scale_and_translate(&mut resampled_points);

        // let board = images.get_mut(&drawingboard.0).expect("Board not found!!");
        // reset_board(window.size(), board, false);
        // for point in resampled_points {
        //     board.set_color_at((point.x + window.size().x/2.0) as u32, (point.y + window.size().y/2.0) as u32, BRUSH_COLOR).unwrap();
        // }

        let end_time = Utc::now();
        let elapsed_time = end_time.signed_duration_since(start_time);
        result_text.0 = format!(
            "{}\n{}.{} milliseconds",
            "PLACEHOLDER",
            elapsed_time.num_milliseconds(),
            elapsed_time.num_microseconds().get_or_insert_default()
        );

        draw_state.0 = DrawMoment::Idle;
        *stroke_index = 0;
    } else if let DrawMoment::Drawing(mouse_pos) = draw_state.0 {
        let board = images.get_mut(&drawingboard.0).expect("Board not found!!");
        let delta = previous_pos.distance(mouse_pos);

        if delta > 6.0 {
            let num_steps = (delta / BRUSH_THICKNESS as f32).ceil() as u32;
            for step in 0..=num_steps {
                let alpha = step as f32 / num_steps as f32;
                let dv = previous_pos.lerp(mouse_pos, alpha);
                fill_pixel(board, dv, false, brush_enabled.0);
            }
        } else {
            fill_pixel(board, mouse_pos, false, brush_enabled.0);
        }

        candidate_vectors[*stroke_index].push(mouse_pos);
        *previous_pos = mouse_pos;
        *total_length += delta;
    }
}

fn spawn(window: Single<&Window>, mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::linear_rgb(0.0, 255.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            ..default()
        },
        ResultText,
    ));

    commands.spawn((
        Text::new("\n\n\n'Toggle Brush' for performance"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::linear_rgb(0.0, 255.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(30.0),
            left: Val::Px(150.0),
            ..default()
        },
    ));

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::End,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(140.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(3.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor(Color::WHITE),
                    BorderRadius::MAX,
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    ToggleBrushButton,
                ))
                .with_child((
                    Text::new("Toggle Brush"),
                    TextFont {
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ToggleBrushButton,
                ));
        });

    let image = Image::new_fill(
        Extent3d {
            width: window.size().x as u32,
            height: window.size().y as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &(BOARD_COLOR.to_srgba().to_u8_array()),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let handle = images.add(image);
    commands.spawn(Sprite::from_image(handle.clone()));
    commands.insert_resource(DrawingBoard(handle));
}

fn setup_window(mut window: Single<&mut Window>) {
    window.title = String::from("Stroke Recognizer");
    window.position = WindowPosition::Centered(MonitorSelection::Current);
}
