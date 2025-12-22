// SPDX-License-Identifier: MPL-2.0
// TODO: Show icon when nothing enabled
// TODO: vertical look and spacing
// TODO: Popup look
// TODO: Code cleanup
use {
    crate::{
        config::{BitrateAppletConfig, Unit},
        fl, network,
    },
    cosmic::{
        self, Element,
        cosmic_config::{self, Config, CosmicConfigEntry},
        iced::{self, Alignment, Length, Limits, Subscription, widget::row, window},
        iced_widget::Row,
        iced_winit::commands::popup::{destroy_popup, get_popup},
        widget::{
            self, autosize, button, container, icon, segmented_button, segmented_control,
            spin_button, toggler,
        },
    },
    std::sync::LazyLock,
    tokio,
};

const APPID: &str = "io.AviralOmar.bitrate";
static AUTOSIZE_MAIN_ID: LazyLock<widget::Id> = LazyLock::new(|| widget::Id::new("autosize-main"));

pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime
    core: cosmic::Core,
    /// The popup id
    popup: Option<window::Id>,
    /// Configuration helper
    config_helper: Config,
    /// Configuration data that persists between application runs
    config: BitrateAppletConfig,
    /// Default network interface
    default_network_interface: Option<String>,
    /// Received bytes
    received_bytes: u64,
    /// Sent bytes
    sent_bytes: u64,
    /// Download speed
    download_speed: u64,
    /// Upload speed
    upload_speed: u64,
    /// Unit model
    unit_model: segmented_button::SingleSelectModel,
    /// Bits Entity
    bits_entity: segmented_button::Entity,
    /// Bytes Entity
    bytes_entity: segmented_button::Entity,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(window::Id),
    UpdateConfig(BitrateAppletConfig),
    UpdateBandwidth,
    UpdateNetworkInterface,
    UnitChanged(segmented_button::Entity),
    UpdateRateChanged(u8),
    ShowDownloadSpeedChanged(bool),
    ShowUploadSpeedChanged(bool),
}

impl AppModel {
    fn format_speed(&self, val: f64) -> String {
        let formatted = if val >= 100.0 {
            format!("{:.1}", val) // e.g., 125.4
        } else if val >= 10.0 {
            format!("{:.2}", val) // e.g., 45.12
        } else {
            format!("{:.2}", val) // e.g., 9.42
        };

        // Clean up trailing zeros
        let result = formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();

        // Final truncation to ensure 5 chars total (optional safety)
        result.chars().take(5).collect()
    }
}

impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "io.AviralOmar.bitrate";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, cosmic::Task<cosmic::Action<Self::Message>>) {
        let config_helper =
            cosmic_config::Config::new(Self::APP_ID, BitrateAppletConfig::VERSION).unwrap();
        let config = cosmic_config::Config::new(Self::APP_ID, BitrateAppletConfig::VERSION)
            .map(|context| match BitrateAppletConfig::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => {
                    // for why in errors {
                    //     tracing::error!(%why, "error loading app config");
                    // }

                    config
                }
            })
            .unwrap_or_default();

        let mut bits_entity = segmented_button::Entity::default();
        let mut bytes_entity = segmented_button::Entity::default();
        let mut unit_model = segmented_button::SingleSelectModel::builder()
            .insert(|b| b.text(fl!("bits")).with_id(|id| bits_entity = id))
            .insert(|b| b.text(fl!("bytes")).with_id(|id| bytes_entity = id))
            .build();

        if config.unit == Unit::Bits {
            unit_model.activate(bits_entity);
        } else if config.unit == Unit::Bytes {
            unit_model.activate(bytes_entity);
        }

        // Set initial received and sent bytes
        let default_network_interface = network::get_default_network_interface();
        let mut received_bytes = 0;
        let mut sent_bytes = 0;
        default_network_interface.inspect(|network_interface| {
            received_bytes = network::get_received_bytes(network_interface).unwrap_or(0);
            sent_bytes = network::get_sent_bytes(network_interface).unwrap_or(0);
        });

        // Construct the app model with the runtime's core.
        let app = AppModel {
            core,
            config_helper,
            config,
            popup: None,
            received_bytes,
            sent_bytes,
            download_speed: 0,
            upload_speed: 0,
            default_network_interface: network::get_default_network_interface(),
            unit_model,
            bits_entity,
            bytes_entity,
        };

        (app, cosmic::Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// The applet's button in the panel will be drawn using the main view method.
    /// This view should emit messages to toggle the applet's popup window, which will
    /// be drawn using the `view_window` method.
    fn view(&self) -> Element<'_, Self::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        // let is_horizontal = self.core.applet.is_horizontal();
        let download_power = if self.download_speed > 0 {
            self.download_speed.ilog2()
        } else {
            0
        };
        let upload_power = if self.upload_speed > 0 {
            self.upload_speed.ilog2()
        } else {
            0
        };
        let download_speed_rebase =
            self.download_speed as f64 / 2u64.pow(download_power - download_power % 10) as f64;
        let download_speed_display = if download_power >= 10 {
            self.format_speed(download_speed_rebase)
        } else {
            format!("{:.0}", download_speed_rebase)
        };
        let upload_speed_rebase =
            self.upload_speed as f64 / 2u64.pow(upload_power - upload_power % 10) as f64;
        let upload_speed_display = if upload_power >= 10 {
            self.format_speed(upload_speed_rebase)
        } else {
            format!("{:.0}", upload_speed_rebase)
        };
        let mut download_unit = String::new();
        if download_power >= 20 {
            download_unit.push('M');
        } else if download_power >= 10 {
            download_unit.push('K');
        }
        let mut upload_unit = String::new();
        if upload_power >= 20 {
            upload_unit.push('M');
        } else if upload_power >= 10 {
            upload_unit.push('K');
        }
        match self.config.unit {
            Unit::Bits => {
                download_unit.push_str("b/s");
                upload_unit.push_str("b/s");
            }
            Unit::Bytes => {
                download_unit.push_str("B/s");
                upload_unit.push_str("B/s");
            }
        }

        let download_wrapper: Element<Message> = container(
            row!(
                container(self.core.applet.text(download_speed_display))
                    .align_right(Length::FillPortion(3)),
                container(
                    row!(
                        self.core.applet.text(download_unit.to_string()),
                        icon::from_name("go-down-symbolic").icon(),
                    )
                    .spacing(cosmic.space_xxs())
                    .align_y(Alignment::Center)
                )
                .align_right(Length::FillPortion(5)),
            )
            .spacing(cosmic.space_xxs())
            .align_y(Alignment::Center),
        )
        .align_right(Length::Fill)
        .into();
        let upload_wrapper: Element<Message> = container(
            row!(
                container(self.core.applet.text(upload_speed_display))
                    .align_right(Length::FillPortion(3)),
                container(
                    row!(
                        self.core.applet.text(upload_unit.to_string()),
                        icon::from_name("go-up-symbolic").icon(),
                    )
                    .spacing(cosmic.space_xxs())
                    .align_y(Alignment::Center)
                )
                .align_right(Length::FillPortion(5)),
            )
            .spacing(cosmic.space_xxs())
            .align_y(Alignment::Center),
        )
        .align_right(Length::Fill)
        .into();
        let mut elements: Vec<Element<Message>> = Vec::new();
        if self.config.show_download_speed {
            elements.push(download_wrapper);
        }
        if self.config.show_upload_speed {
            elements.push(upload_wrapper);
        }
        let widget_width;
        if self.config.show_download_speed && self.config.show_upload_speed {
            widget_width = Length::Fixed((self.core.applet.suggested_size(true).0 * 12) as f32);
        } else if self.config.show_download_speed || self.config.show_upload_speed {
            widget_width = Length::Fixed((self.core.applet.suggested_size(true).0 * 7) as f32);
        } else {
            widget_width = Length::Fixed((self.core.applet.suggested_size(true).0) as f32);
            elements.push(icon::from_name(APPID).into());
        }
        let wrapper: Element<Message> = Row::from_vec(elements)
            .height(Length::Fixed(
                (self.core.applet.suggested_size(true).1
                    + 2 * self.core.applet.suggested_padding(true).1) as f32,
            ))
            .width(widget_width)
            .padding([0, self.core.applet.suggested_padding(true).0])
            .spacing(cosmic.space_xxs())
            .align_y(Alignment::Center)
            .into();
        let button = button::custom(wrapper)
            .on_press_down(Message::TogglePopup)
            .class(cosmic::theme::Button::AppletIcon);
        autosize::autosize(container(button), AUTOSIZE_MAIN_ID.clone()).into()
    }

    /// The applet's popup window will be drawn using this view method. If there are
    /// multiple poups, you may match the id parameter to determine which popup to
    /// create a view for.
    fn view_window(&self, _id: window::Id) -> Element<'_, Self::Message> {
        let content_list = widget::list_column()
            .padding(5)
            .spacing(0)
            .add(widget::settings::item(
                fl!("unit"),
                segmented_control::horizontal(&self.unit_model).on_activate(Message::UnitChanged),
            ))
            .add(widget::settings::item(
                fl!("update-rate"),
                spin_button::spin_button(
                    format!("{} s", self.config.update_rate),
                    self.config.update_rate,
                    1,
                    1,
                    10,
                    Message::UpdateRateChanged,
                ),
            ))
            .add(
                toggler(self.config.show_download_speed)
                    .label(fl!("show-download-speed"))
                    .on_toggle(Message::ShowDownloadSpeedChanged),
            )
            .add(
                toggler(self.config.show_upload_speed)
                    .label(fl!("show-upload-speed"))
                    .on_toggle(Message::ShowUploadSpeedChanged),
            );

        self.core.applet.popup_container(content_list).into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-lived async tasks running in the background which
    /// emit messages to the application through a channel. They may be conditionally
    /// activated by selectively appending to the subscription batch, and will
    /// continue to execute for the duration that they remain in the batch.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            (iced::time::every(tokio::time::Duration::from_secs(
                self.config.update_rate as u64,
            )))
            .map(|_| Message::UpdateBandwidth),
            (iced::time::every(tokio::time::Duration::from_secs(5)))
                .map(|_| Message::UpdateNetworkInterface),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<BitrateAppletConfig>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime. The application will not exit until all
    /// tasks are finished.
    fn update(&mut self, message: Self::Message) -> cosmic::Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateBandwidth => {
                if let Some(network_interface) = &self.default_network_interface {
                    if let Some(received_bytes_cur) = network::get_received_bytes(network_interface)
                    {
                        self.download_speed = received_bytes_cur - self.received_bytes;
                        if self.config.unit == Unit::Bits {
                            self.download_speed *= 8;
                        }
                        self.download_speed /= self.config.update_rate as u64;
                        self.received_bytes = received_bytes_cur;
                    }
                    if let Some(sent_bytes_cur) = network::get_sent_bytes(network_interface) {
                        self.upload_speed = sent_bytes_cur - self.sent_bytes;
                        if self.config.unit == Unit::Bits {
                            self.upload_speed *= 8;
                        }
                        self.upload_speed /= self.config.update_rate as u64;
                        self.sent_bytes = sent_bytes_cur;
                    }
                }
            }
            Message::UpdateNetworkInterface => {
                self.default_network_interface = network::get_default_network_interface();
            }
            Message::UnitChanged(entity) => {
                if !self.unit_model.is_active(entity) {
                    self.unit_model.activate(entity);
                    if entity == self.bits_entity {
                        self.download_speed *= 8;
                        self.upload_speed *= 8;
                        self.config
                            .set_unit(&self.config_helper, Unit::Bits)
                            .unwrap();
                    } else if entity == self.bytes_entity {
                        self.download_speed /= 8;
                        self.upload_speed /= 8;
                        self.config
                            .set_unit(&self.config_helper, Unit::Bytes)
                            .unwrap();
                    }
                }
            }
            Message::UpdateRateChanged(rate) => {
                self.config
                    .set_update_rate(&self.config_helper, rate)
                    .unwrap();
            }
            Message::ShowDownloadSpeedChanged(show) => {
                self.config
                    .set_show_download_speed(&self.config_helper, show)
                    .unwrap();
            }
            Message::ShowUploadSpeedChanged(show) => {
                self.config
                    .set_show_upload_speed(&self.config_helper, show)
                    .unwrap();
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = window::Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        cosmic::Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
