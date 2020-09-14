use crate::app::App;
use crate::common::Warping;
use crate::game::{PopupMsg, Transition};
use crate::helpers::ID;
use crate::sandbox::time_warp::JumpToTime;
use crate::sandbox::{GameplayMode, SandboxMode, TimeWarpScreen};
use geom::{Duration, Polygon, Time};
use sim::AlertLocation;
use widgetry::{
    hotkey, Btn, Choice, Color, EventCtx, GeomBatch, GfxCtx, HorizontalAlignment, Key, Line,
    Outcome, Panel, PersistentSplit, RewriteColor, Text, VerticalAlignment, Widget,
};

pub struct SpeedControls {
    pub panel: Panel,

    paused: bool,
    setting: SpeedSetting,
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum SpeedSetting {
    // 1 sim second per real second
    Realtime,
    // 5 sim seconds per real second
    Fast,
    // 30 sim seconds per real second
    Faster,
    // 1 sim hour per real second
    Fastest,
}

impl SpeedControls {
    fn make_panel(ctx: &mut EventCtx, app: &App, paused: bool, setting: SpeedSetting) -> Panel {
        let mut row = Vec::new();
        row.push(
            if paused {
                Btn::svg_def("system/assets/speed/triangle.svg").build(
                    ctx,
                    "play",
                    hotkey(Key::Space),
                )
            } else {
                Btn::svg_def("system/assets/speed/pause.svg").build(
                    ctx,
                    "pause",
                    hotkey(Key::Space),
                )
            }
            .container()
            .padding(9)
            .bg(app.cs.section_bg)
            .margin_right(16),
        );

        row.push(
            Widget::custom_row(
                vec![
                    (SpeedSetting::Realtime, "real-time speed"),
                    (SpeedSetting::Fast, "5x speed"),
                    (SpeedSetting::Faster, "30x speed"),
                    (SpeedSetting::Fastest, "3600x speed"),
                ]
                .into_iter()
                .map(|(s, label)| {
                    let mut txt = Text::from(Line(label).small());
                    txt.extend(Text::tooltip(ctx, hotkey(Key::LeftArrow), "slow down"));
                    txt.extend(Text::tooltip(ctx, hotkey(Key::RightArrow), "speed up"));

                    GeomBatch::load_svg(ctx.prerender, "system/assets/speed/triangle.svg")
                        .color(if setting >= s {
                            RewriteColor::NoOp
                        } else {
                            RewriteColor::ChangeAll(Color::WHITE.alpha(0.2))
                        })
                        .to_btn(ctx)
                        .tooltip(txt)
                        .build(ctx, label, None)
                        .margin_right(6)
                })
                .collect(),
            )
            .bg(app.cs.section_bg)
            .centered()
            .padding(6)
            .margin_right(16),
        );

        row.push(
            PersistentSplit::new(
                ctx,
                "step forwards",
                app.opts.time_increment,
                hotkey(Key::M),
                vec![
                    Choice::new("+1h", Duration::hours(1)),
                    Choice::new("+30m", Duration::minutes(30)),
                    Choice::new("+10m", Duration::minutes(10)),
                    Choice::new("+0.1s", Duration::seconds(0.1)),
                ],
            )
            .bg(app.cs.section_bg)
            .margin_right(16),
        );

        row.push(
            Widget::custom_row(vec![
                Btn::svg_def("system/assets/speed/jump_to_time.svg")
                    .build(ctx, "jump to specific time", hotkey(Key::B))
                    .container()
                    .padding(9),
                Btn::svg_def("system/assets/speed/reset.svg")
                    .build(ctx, "reset to midnight", hotkey(Key::X))
                    .container()
                    .padding(9),
            ])
            .bg(app.cs.section_bg),
        );

        Panel::new(Widget::custom_row(row))
            .aligned(
                HorizontalAlignment::Center,
                VerticalAlignment::BottomAboveOSD,
            )
            .build(ctx)
    }

    pub fn new(ctx: &mut EventCtx, app: &App) -> SpeedControls {
        let panel = SpeedControls::make_panel(ctx, app, false, SpeedSetting::Realtime);
        SpeedControls {
            panel,
            paused: false,
            setting: SpeedSetting::Realtime,
        }
    }

    pub fn event(
        &mut self,
        ctx: &mut EventCtx,
        app: &mut App,
        maybe_mode: Option<&GameplayMode>,
    ) -> Option<Transition> {
        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "real-time speed" => {
                    self.setting = SpeedSetting::Realtime;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                    return None;
                }
                "5x speed" => {
                    self.setting = SpeedSetting::Fast;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                    return None;
                }
                "30x speed" => {
                    self.setting = SpeedSetting::Faster;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                    return None;
                }
                "3600x speed" => {
                    self.setting = SpeedSetting::Fastest;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                    return None;
                }
                "play" => {
                    self.paused = false;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                    return None;
                }
                "pause" => {
                    self.pause(ctx, app);
                }
                "reset to midnight" => {
                    if let Some(mode) = maybe_mode {
                        return Some(Transition::Replace(SandboxMode::new(
                            ctx,
                            app,
                            mode.clone(),
                        )));
                    } else {
                        return Some(Transition::Push(PopupMsg::new(
                            ctx,
                            "Error",
                            vec!["Sorry, you can't go rewind time from this mode."],
                        )));
                    }
                }
                "jump to specific time" => {
                    return Some(Transition::Push(JumpToTime::new(
                        ctx,
                        app,
                        maybe_mode.cloned(),
                    )));
                }
                "step forwards" => {
                    let dt = self.panel.persistent_split_value("step forwards");
                    if dt == Duration::seconds(0.1) {
                        app.primary
                            .sim
                            .tiny_step(&app.primary.map, &mut app.primary.sim_cb);
                        app.recalculate_current_selection(ctx);
                        return Some(Transition::KeepWithMouseover);
                    }
                    return Some(Transition::Push(TimeWarpScreen::new(
                        ctx,
                        app,
                        app.primary.sim.time() + dt,
                        None,
                    )));
                }
                _ => unreachable!(),
            },
            _ => {}
        }
        // Just kind of constantly scrape this
        app.opts.time_increment = self.panel.persistent_split_value("step forwards");

        if ctx.input.key_pressed(Key::LeftArrow) {
            match self.setting {
                SpeedSetting::Realtime => self.pause(ctx, app),
                SpeedSetting::Fast => {
                    self.setting = SpeedSetting::Realtime;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                }
                SpeedSetting::Faster => {
                    self.setting = SpeedSetting::Fast;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                }
                SpeedSetting::Fastest => {
                    self.setting = SpeedSetting::Faster;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                }
            }
        }
        if ctx.input.key_pressed(Key::RightArrow) {
            match self.setting {
                SpeedSetting::Realtime => {
                    if self.paused {
                        self.paused = false;
                        self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                    } else {
                        self.setting = SpeedSetting::Fast;
                        self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                    }
                }
                SpeedSetting::Fast => {
                    self.setting = SpeedSetting::Faster;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                }
                SpeedSetting::Faster => {
                    self.setting = SpeedSetting::Fastest;
                    self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
                }
                SpeedSetting::Fastest => {}
            }
        }

        if !self.paused {
            if let Some(real_dt) = ctx.input.nonblocking_is_update_event() {
                ctx.input.use_update_event();
                let multiplier = match self.setting {
                    SpeedSetting::Realtime => 1.0,
                    SpeedSetting::Fast => 5.0,
                    SpeedSetting::Faster => 30.0,
                    SpeedSetting::Fastest => 3600.0,
                };
                let dt = multiplier * real_dt;
                // TODO This should match the update frequency in widgetry. Plumb along the deadline
                // or frequency to here.
                app.primary.sim.time_limited_step(
                    &app.primary.map,
                    dt,
                    Duration::seconds(0.033),
                    &mut app.primary.sim_cb,
                );
                app.recalculate_current_selection(ctx);
            }
        }

        // TODO Need to do this anywhere that steps the sim, like TimeWarpScreen.
        let alerts = app.primary.sim.clear_alerts();
        if !alerts.is_empty() {
            let popup = PopupMsg::new(
                ctx,
                "Alerts",
                alerts.iter().map(|(_, _, msg)| msg).collect(),
            );
            let maybe_id = match alerts[0].1 {
                AlertLocation::Nil => None,
                AlertLocation::Intersection(i) => Some(ID::Intersection(i)),
                // TODO Open info panel and warp to them
                AlertLocation::Person(_) => None,
                AlertLocation::Building(b) => Some(ID::Building(b)),
            };
            // TODO Can filter for particular alerts places like this:
            /*if !alerts[0].2.contains("Turn conflict cycle") {
                return None;
            }*/
            /*if maybe_id != Some(ID::Building(map_model::BuildingID(91))) {
                return None;
            }*/
            self.pause(ctx, app);
            if let Some(id) = maybe_id {
                // Just go to the first one, but print all messages
                return Some(Transition::Multi(vec![
                    Transition::Push(popup),
                    Transition::Push(Warping::new(
                        ctx,
                        id.canonical_point(&app.primary).unwrap(),
                        Some(10.0),
                        None,
                        &mut app.primary,
                    )),
                ]));
            } else {
                return Some(Transition::Push(popup));
            }
        }

        None
    }

    pub fn draw(&self, g: &mut GfxCtx) {
        self.panel.draw(g);
    }

    pub fn pause(&mut self, ctx: &mut EventCtx, app: &App) {
        if !self.paused {
            self.paused = true;
            self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
        }
    }

    pub fn resume_realtime(&mut self, ctx: &mut EventCtx, app: &App) {
        if self.paused || self.setting != SpeedSetting::Realtime {
            self.paused = false;
            self.setting = SpeedSetting::Realtime;
            self.panel = SpeedControls::make_panel(ctx, app, self.paused, self.setting);
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

pub struct TimePanel {
    time: Time,
    pub panel: Panel,
}

impl TimePanel {
    pub fn new(ctx: &mut EventCtx, app: &App) -> TimePanel {
        TimePanel {
            time: app.primary.sim.time(),
            panel: Panel::new(Widget::col(vec![
                Text::from(Line(app.primary.sim.time().ampm_tostring()).big_monospaced())
                    .draw(ctx)
                    .centered_horiz(),
                {
                    let mut batch = GeomBatch::new();
                    // This is manually tuned
                    let width = 300.0;
                    let height = 15.0;
                    // Just clamp if we simulate past the expected end
                    let percent = app
                        .primary
                        .sim
                        .time()
                        .to_percent(app.primary.sim.get_end_of_day())
                        .min(1.0);

                    // TODO Why is the rounding so hard? The white background is always rounded
                    // at both ends. The moving bar should always be rounded on the left, flat
                    // on the right, except at the very end (for the last 'radius' pixels). And
                    // when the width is too small for the radius, this messes up.

                    batch.push(Color::WHITE, Polygon::rectangle(width, height));

                    if percent != 0.0 {
                        batch.push(
                            if percent < 0.25 || percent > 0.75 {
                                app.cs.night_time_slider
                            } else {
                                app.cs.day_time_slider
                            },
                            Polygon::rectangle(percent * width, height),
                        );
                    }

                    Widget::draw_batch(ctx, batch)
                },
                Widget::custom_row(vec![
                    Line("00:00").small_monospaced().draw(ctx),
                    Widget::draw_svg(ctx, "system/assets/speed/sunrise.svg"),
                    Line("12:00").small_monospaced().draw(ctx),
                    Widget::draw_svg(ctx, "system/assets/speed/sunset.svg"),
                    Line("24:00").small_monospaced().draw(ctx),
                ])
                .evenly_spaced(),
            ]))
            .aligned(HorizontalAlignment::Left, VerticalAlignment::Top)
            .build(ctx),
        }
    }

    pub fn event(&mut self, ctx: &mut EventCtx, app: &mut App) {
        if self.time != app.primary.sim.time() {
            *self = TimePanel::new(ctx, app);
        }
        self.panel.event(ctx);
    }

    pub fn draw(&self, g: &mut GfxCtx) {
        self.panel.draw(g);
    }
}
