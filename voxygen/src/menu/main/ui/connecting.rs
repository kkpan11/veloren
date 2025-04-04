use super::{ConnectionState, Imgs, Message};

use crate::{
    game_input::GameInput,
    menu::main::DetailedInitializationStage,
    settings::ControlSettings,
    ui::{
        Graphic,
        fonts::IcedFonts as Fonts,
        ice::{Element, IcedUi as Ui, Id, component::neat_button, style, widget::Image},
    },
};
use client::ClientInitStage;
use common::assets::{self, AssetExt};
use i18n::Localization;
use iced::{Align, Column, Container, Length, Row, Space, Text, button};
use keyboard_keynames::key_layout::KeyLayout;
use serde::{Deserialize, Serialize};
#[cfg(feature = "singleplayer")]
use server::{ServerInitStage, WorldCivStage, WorldGenerateStage, WorldSimStage};

struct LoadingAnimation {
    speed_factor: f32,
    frames: Vec<Id>,
}
impl LoadingAnimation {
    fn new(raw: &(f32, Vec<String>), ui: &mut Ui) -> Self {
        let mut frames = vec![];
        for frame_path in raw.1.iter() {
            if let Ok(image) = assets::Image::load(frame_path) {
                frames.push(ui.add_graphic(Graphic::Image(image.read().to_image(), None)));
            } else {
                frames.push(
                    ui.add_graphic(Graphic::Image(
                        assets::Image::load("voxygen.element.not_found")
                            .unwrap_or_else(|_| panic!("Missing asset '{}'", frame_path))
                            .read()
                            .to_image(),
                        None,
                    )),
                )
            }
        }
        Self {
            speed_factor: raw.0,
            frames,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct LoadingAnimationManifest(Vec<(f32, Vec<String>)>);
impl assets::Asset for LoadingAnimationManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

/// Connecting screen for the main menu
pub struct Screen {
    cancel_button: button::State,
    add_button: button::State,
    tip_number: u16,
    loading_animation: LoadingAnimation,
}

impl Screen {
    pub fn new(ui: &mut Ui) -> Self {
        let animations =
            LoadingAnimationManifest::load("voxygen.element.animation.loaders.manifest")
                .expect(
                    "Missing loader manifest file 'voxygen/element/animation/loaders/manifest.ron'",
                )
                .cloned()
                .0;
        Self {
            cancel_button: Default::default(),
            add_button: Default::default(),
            tip_number: rand::random(),
            loading_animation: LoadingAnimation::new(
                &animations[rand::random::<usize>() % animations.len()],
                ui,
            ),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        connection_state: &ConnectionState,
        init_stage: &DetailedInitializationStage,
        time: f64,
        i18n: &Localization,
        button_style: style::button::Style,
        show_tip: bool,
        controls: &ControlSettings,
        key_layout: &Option<KeyLayout>,
    ) -> Element<Message> {
        // TODO: add built in support for animated images
        let frame_index = (time * self.loading_animation.speed_factor as f64)
            % self.loading_animation.frames.len() as f64;
        let frame_id = self.loading_animation.frames[frame_index as usize];

        let children = match connection_state {
            ConnectionState::InProgress => {
                let tip = if show_tip {
                    let key = |code| match controls.keybindings.get(&code) {
                        Some(Some(key_mouse)) => key_mouse.display_string(key_layout),
                        Some(None) => i18n.get_msg("main-unbound_key_tip").into_owned(),
                        None => match ControlSettings::default_binding(code) {
                            Some(key_mouse) => key_mouse.display_string(key_layout),
                            None => i18n.get_msg("main-unbound_key_tip").into_owned(),
                        },
                    };
                    let keys = i18n::fluent_args! {
                        "gameinput-togglelantern" => key(GameInput::ToggleLantern),
                        "gameinput-controls" => key(GameInput::Controls),
                        "gameinput-settings" => key(GameInput::Settings),
                        "gameinput-social" => key(GameInput::Social),
                        "gameinput-dance" => key(GameInput::Dance),
                        "gameinput-glide" => key(GameInput::Glide),
                        "gameinput-sit" => key(GameInput::Sit),
                        "gameinput-crafting" => key(GameInput::Crafting),
                        "gameinput-roll" => key(GameInput::Roll),
                        "gameinput-screenshot" => key(GameInput::Screenshot),
                    };
                    let tip = &i18n.get_variation_ctx("loading-tips", self.tip_number, &keys);
                    let tip = format!("{} {}", i18n.get_msg("main-tip"), tip);

                    Container::new(
                        Text::new(tip)
                            .horizontal_alignment(iced::HorizontalAlignment::Center)
                            .size(fonts.cyri.scale(25)),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .align_y(Align::End)
                    .into()
                } else {
                    Space::new(Length::Fill, Length::Fill).into()
                };

                let stage = {
                    let stage_message = match init_stage {
                        #[cfg(feature = "singleplayer")]
                        DetailedInitializationStage::Singleplayer => {
                            i18n.get_msg("hud-init-stage-singleplayer")
                        },
                        #[cfg(feature = "singleplayer")]
                        DetailedInitializationStage::SingleplayerServer(server_stage) => {
                            match server_stage {
                                ServerInitStage::DbMigrations => {
                                    i18n.get_msg("hud-init-stage-server-db-migrations")
                                },
                                ServerInitStage::DbVacuum => {
                                    i18n.get_msg("hud-init-stage-server-db-vacuum")
                                },
                                ServerInitStage::WorldGen(worldgen_stage) => match worldgen_stage {
                                    WorldGenerateStage::WorldSimGenerate(worldsim_stage) => {
                                        match worldsim_stage {
                                            WorldSimStage::Erosion { progress, estimate } => {
                                                let mut msg = i18n
                                                .get_msg_ctx(
                                                    "hud-init-stage-server-worldsim-erosion",
                                                    &i18n::fluent_args! { "percentage" => format!("{progress:.0}") }
                                                ).into_owned();
                                                if let Some(estimate) = estimate {
                                                    let (attr, duration) =
                                                        chrono::Duration::from_std(*estimate)
                                                            .map(|dur| {
                                                                let days = dur.num_days();
                                                                if days > 0 {
                                                                    return ("days", days);
                                                                }
                                                                let hours = dur.num_hours();
                                                                if hours > 0 {
                                                                    return ("hours", hours);
                                                                }
                                                                let minutes = dur.num_minutes();
                                                                if minutes > 0 {
                                                                    return ("minutes", minutes);
                                                                }

                                                                ("seconds", dur.num_seconds())
                                                            })
                                                            .unwrap_or(("days", i64::MAX));
                                                    msg.push(' ');
                                                    msg.push('(');
                                                    msg.push_str(&i18n.get_attr_ctx(
                                                        "hud-init-stage-server-worldsim-erosion_time_left",
                                                        attr,
                                                        &i18n::fluent_args! { "n" => duration }
                                                    ));
                                                    msg.push(')');
                                                }

                                                std::borrow::Cow::Owned(msg)
                                            },
                                        }
                                    },
                                    WorldGenerateStage::WorldCivGenerate(worldciv_stage) => {
                                        match worldciv_stage {
                                            WorldCivStage::CivCreation(generated, total) => i18n
                                                .get_msg_ctx(
                                                    "hud-init-stage-server-worldciv-civcreate",
                                                    &i18n::fluent_args! {
                                                        "generated" => generated.to_string(),
                                                        "total" => total.to_string(),
                                                    },
                                                ),
                                            WorldCivStage::SiteGeneration => {
                                                i18n.get_msg("hud-init-stage-server-worldciv-site")
                                            },
                                        }
                                    },
                                    WorldGenerateStage::EconomySimulation => {
                                        i18n.get_msg("hud-init-stage-server-economysim")
                                    },
                                    WorldGenerateStage::SpotGeneration => {
                                        i18n.get_msg("hud-init-stage-server-spotgen")
                                    },
                                },
                                ServerInitStage::StartingSystems => {
                                    i18n.get_msg("hud-init-stage-server-starting")
                                },
                            }
                        },
                        DetailedInitializationStage::StartingMultiplayer => {
                            i18n.get_msg("hud-init-stage-multiplayer")
                        },
                        DetailedInitializationStage::Client(client_stage) => match client_stage {
                            ClientInitStage::ConnectionEstablish => {
                                i18n.get_msg("hud-init-stage-client-connection-establish")
                            },
                            ClientInitStage::WatingForServerVersion => {
                                i18n.get_msg("hud-init-stage-client-request-server-version")
                            },
                            ClientInitStage::Authentication => {
                                i18n.get_msg("hud-init-stage-client-authentication")
                            },
                            ClientInitStage::LoadingInitData => {
                                i18n.get_msg("hud-init-stage-client-load-init-data")
                            },
                            ClientInitStage::StartingClient => {
                                i18n.get_msg("hud-init-stage-client-starting-client")
                            },
                        },
                        DetailedInitializationStage::CreatingRenderPipeline(done, total) => i18n
                            .get_msg_ctx(
                                "hud-init-stage-render-pipeline",
                                &i18n::fluent_args! { "done" => done, "total" => total },
                            ),
                    };

                    Container::new(Text::new(stage_message).size(fonts.cyri.scale(20)))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(10)
                        .align_x(Align::Start)
                        .into()
                };

                let cancel = Container::new(neat_button(
                    &mut self.cancel_button,
                    i18n.get_msg("common-cancel"),
                    0.7,
                    button_style,
                    Some(Message::CancelConnect),
                ))
                .width(Length::Fill)
                .height(Length::Units(fonts.cyri.scale(30)))
                .center_x()
                .padding(3);

                let tip_cancel = Column::with_children(vec![tip, cancel.into()])
                    .width(Length::FillPortion(2))
                    .align_items(Align::Center)
                    .spacing(5)
                    .padding(5);

                let gear = Container::new(
                    Image::new(frame_id)
                        .width(Length::Units(64))
                        .height(Length::Units(64)),
                )
                .width(Length::Fill)
                .padding(10)
                .align_x(Align::End);

                let bottom_content =
                    Row::with_children(vec![stage, tip_cancel.into(), gear.into()])
                        .align_items(Align::Center)
                        .width(Length::Fill);

                let left_art = Image::new(imgs.loading_art_l)
                    .width(Length::Units(12))
                    .height(Length::Units(12));
                let right_art = Image::new(imgs.loading_art_r)
                    .width(Length::Units(12))
                    .height(Length::Units(12));

                let bottom_bar = Container::new(Row::with_children(vec![
                    left_art.into(),
                    bottom_content.into(),
                    right_art.into(),
                ]))
                .height(Length::Units(85))
                .style(style::container::Style::image(imgs.loading_art));

                vec![
                    Space::new(Length::Fill, Length::Fill).into(),
                    bottom_bar.into(),
                ]
            },
            ConnectionState::AuthTrustPrompt { msg, .. } => {
                let text = Text::new(msg).size(fonts.cyri.scale(25));

                let cancel = neat_button(
                    &mut self.cancel_button,
                    i18n.get_msg("common-cancel"),
                    0.7,
                    button_style,
                    Some(Message::TrustPromptCancel),
                );
                let add = neat_button(
                    &mut self.add_button,
                    i18n.get_msg("common-add"),
                    0.7,
                    button_style,
                    Some(Message::TrustPromptAdd),
                );

                let content = Column::with_children(vec![
                    text.into(),
                    Container::new(
                        Row::with_children(vec![cancel, add])
                            .spacing(20)
                            .height(Length::Units(25)),
                    )
                    .align_x(Align::End)
                    .width(Length::Fill)
                    .into(),
                ])
                .spacing(4)
                .max_width(520)
                .width(Length::Fill)
                .height(Length::Fill);

                let prompt_window = Container::new(content)
                    .style(
                        style::container::Style::color_with_double_cornerless_border(
                            (22, 18, 16, 255).into(),
                            (11, 11, 11, 255).into(),
                            (54, 46, 38, 255).into(),
                        ),
                    )
                    .padding(20);

                let container = Container::new(prompt_window)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y();

                vec![
                    container.into(),
                    Space::new(Length::Fill, Length::Units(fonts.cyri.scale(15))).into(),
                ]
            },
        };

        Column::with_children(children)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
