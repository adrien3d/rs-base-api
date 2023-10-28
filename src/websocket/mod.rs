pub mod handlers;

use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_http::ws::{Item, Message};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use anyhow::Result;
use bson::Document;
use futures::Stream;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::time::{Duration, Instant};

use crate::{ProgramAppState, MAX_FRAME_SIZE};

/// How long before lack of client response causes a timeout.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// Should be half (or less) of the acceptable client timeout.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[repr(u8)]
#[derive(Debug, IntoPrimitive, TryFromPrimitive, PartialEq, Eq, Clone, Copy)]
/// The kind of an incoming request via the websocket.
pub enum RequestId {
    BasicCommand = 0,
}

/// An active websocket connection.
pub struct WebsocketConnection {
    /// The state of Api, containing Ntp, MongoDB & UI sender channel.
    state: web::Data<ProgramAppState>,
    /// Client must send ping at least once [`CLIENT_TIMEOUT`], otherwise we drop the connection.
    pub last_heartbeat: Instant,
    /// The contents of a fragmented message that is currently being received.
    pub current_fragmented_message: Option<Vec<u8>>,
    /// The address of the other side of this websocket.
    address: String,
}

impl Actor for WebsocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        log::info!("Accepted websocket connection from {}", self.address);
        // Register the websocket heartbeat manager when the connection is established.
        context.run_interval(HEARTBEAT_INTERVAL, |connection, context| {
            // Check client heartbeats
            if Instant::now().duration_since(connection.last_heartbeat) <= CLIENT_TIMEOUT {
                context.ping(b"");
                return;
            }
            if !context.state().stopping() {
                log::warn!(
                    "Client {} has not sent heartbeat in over {CLIENT_TIMEOUT:?}, disconnecting",
                    connection.address
                );
            }
            context.stop();
        });

        // Start outgoing response task.
        context.add_stream(handle_response(self.state.clone()));
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        log::info!("Dropped websocket connection with {}", self.address);
    }
}

fn handle_response(
    state: web::Data<ProgramAppState>,
) -> impl Stream<Item = Result<ws::Message, ws::ProtocolError>> {
    async_stream::stream! {
        let mut ui_output_channel = state.ui_sender_channel.subscribe();
        loop {
            match ui_output_channel.recv().await {
                Err(error) => log::warn!("Failed to receive UI output message: {error}"),
                Ok(message) => yield Ok(Message::Binary(message.into())),
            }
        }
    }
}

/// WebSocket message handler.
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketConnection {
    fn handle(
        &mut self,
        message: Result<ws::Message, ws::ProtocolError>,
        context: &mut Self::Context,
    ) {
        let message = match message {
            Err(error) => {
                log::warn!("Websocket stream error, dropping connection: {error}");
                context.stop();
                return;
            }
            Ok(message) => message,
        };
        log::debug!("Websocket message: {message:?}");
        match message {
            ws::Message::Ping(msg) => {
                self.last_heartbeat = Instant::now();
                context.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.last_heartbeat = Instant::now();
            }
            ws::Message::Binary(data) => match Document::from_reader(data.as_ref()) {
                Ok(request) => handle_requests(&self.state, request),
                Err(error) => {
                    log::warn!("Ignoring invalid BSON package: {error}");
                }
            },
            ws::Message::Close(reason) => {
                context.close(reason);
                context.stop();
            }
            ws::Message::Continuation(continuation) => {
                match continuation {
                    Item::FirstText(_) => {}
                    Item::FirstBinary(data) => {
                        let mut buffer = if let Some(mut buffer) =
                            self.current_fragmented_message.take()
                        {
                            log::warn!("Received new fragmented message while waiting for chunks of other fragmented message. Discarding older message.");
                            buffer.clear();
                            buffer
                        } else {
                            Vec::new()
                        };
                        buffer.extend_from_slice(&data);
                        self.current_fragmented_message = Some(buffer);
                    }
                    Item::Continue(data) => {
                        let Some(ref mut buffer) = self.current_fragmented_message else {
                            log::warn!("Received part of a fragmented message without the start, ignoring it.");
                            return;
                        };
                        buffer.extend_from_slice(&data);
                    }
                    Item::Last(data) => {
                        let Some(ref mut buffer) = self.current_fragmented_message else {
                            log::warn!("Received end of a fragmented message without the start, ignoring it.");
                            return;
                        };
                        buffer.extend_from_slice(&data);
                        match Document::from_reader(buffer.as_slice()) {
                            Ok(request) => handle_requests(&self.state, request),
                            Err(error) => {
                                log::warn!("Ignoring invalid BSON package: {error}");
                            }
                        }
                        self.current_fragmented_message = None;
                    }
                }
            }
            ws::Message::Text(_) | ws::Message::Nop => {}
        }
    }
}

fn handle_requests(state: &web::Data<ProgramAppState>, request: Document) {
    let id: RequestId = match request.get_i32("id") {
        Err(error) => {
            log::warn!("Ignoring package with missing or wrongly typed id in request: {error}");
            return;
        }

        Ok(value) => match value
            .try_into()
            .ok()
            .and_then(|value: u8| RequestId::try_from(value).ok())
        {
            Some(id) => id,
            None => {
                log::warn!("Ignoring package with invalid id: {value}");
                return;
            }
        },
    };
    let data = match request.get_document("data") {
        Ok(data) => data,
        Err(error) => {
            log::warn!("Ignoring package with invalid data: {error}");
            return;
        }
    };
    let result = match id {
        RequestId::BasicCommand => {
            bson::from_bson(data.into()).map(|data| handlers::basic_command(state, data))
        }
    };
    match result {
        Ok(Ok(_)) => {
            log::debug!("Request from {id:?} route went OK");
        }
        Ok(Err(error)) => {
            log::error!("Request from {id:?} failed with error ({error})");
            // state.response.send_error(ApiError::UnknownId);
        }
        Err(error) => {
            log::error!("Failed to parse data for request {id:?}: {error:?}");
            // state.response.send_error(ApiError::TelecommandParseError);
        }
    }
}

/// Handshake and start WebSocket handler.
pub async fn handle_ws(
    request: HttpRequest,
    stream: web::Payload,
    state: web::Data<ProgramAppState>,
) -> Result<HttpResponse, Error> {
    ws::WsResponseBuilder::new(
        WebsocketConnection {
            state,
            last_heartbeat: Instant::now(),
            current_fragmented_message: None,
            address: request
                .peer_addr()
                .map_or_else(|| String::from("<?>"), |address| address.to_string()),
        },
        &request,
        stream,
    )
    .frame_size(MAX_FRAME_SIZE)
    .start()
}
