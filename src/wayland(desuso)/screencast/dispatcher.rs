use wayland_client::{Dispatch, protocol::wl_registry};

use crate::utils::logging::LOGGER;
use crate::wayland::zkde_screencast;
use crate::wayland::zkde_screencast::zkde_screencast_unstable_v1::{ZkdeScreencastUnstableV1};
use crate::wayland::zkde_screencast::__interfaces::ZKDE_SCREENCAST_UNSTABLE_V1_INTERFACE;
use crate::wayland::zkde_screencast::zkde_screencast_stream_unstable_v1::{ZkdeScreencastStreamUnstableV1};

use zkde_screencast::zkde_screencast_stream_unstable_v1::Event as ScreenCastStreamEvents;

use super::state::ScreenCastState;
use std::io::Write;


/*
    Example usage:
    use crate::wayland::{screencast::state::ScreenCastState, zkde_screencast::Pointer};
    {
        let conn = wayland_client::Connection::connect_to_env().expect("Error obteniendo conexión");
        let mut event_queue = conn.new_event_queue();
        let qh = event_queue.handle();

        let display = conn.display();
        display.get_registry(&qh, ());

        let mut state = ScreenCastState::new();

        // Sincronizar eventos para obtener el registro global
        event_queue.roundtrip(&mut state).expect("Error obteniendo el registro global");

        if let Some(screencast) = state.screencast_manager.take() {
            let mut input: String = String::new();
            println!("Ingrese el UUID de la ventana a capturar (ej: f630d34f-463a-4743-923c-cb829514f7a8): ");
            std::io::stdin() 
                .read_line(&mut input) 
                .expect("Unable to read Stdin");

            let window_uuid = String::from(input.trim());
            let pointer_mode = Pointer::Embedded as u32;

            // Llamada al método stream_window:
            // Devuelve el proxy ZkdeScreencastStreamV1 registrado en la event queue.
            let stream = screencast.stream_window(
                window_uuid,
                pointer_mode,
                &qh,
                (),
            );

            let mut logger = LOGGER.lock().await;
            writeln!(logger, "Obtenido streaming con node.id = {:#?}", stream.id())?;

            state.stream = Some(stream);

            // Escuchar eventos en bucle (esperando el evento `Created` con el ID de PipeWire)
        loop {
            event_queue.blocking_dispatch(&mut state).expect("Error escuchando eventos...");
        }

        }
    }

*/

// 0. No sirve para nada, pero es necesario para el siguiente dispatcher

impl Dispatch<ZkdeScreencastUnstableV1, ()> for ScreenCastState {

    #![allow(warnings)]
    fn event(
        state: &mut Self,
        proxy: &ZkdeScreencastUnstableV1,
        event: <ZkdeScreencastUnstableV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // La interfaz principal no suele emitir eventos directos
    }

}


// 1. Manejo de la registry para descubrir e instanciar la interfaz de KDE (Request -> Mensaje de cliente a servidor)

impl Dispatch<wl_registry::WlRegistry, ()> for ScreenCastState {

    #![allow(warnings)]
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        
        if let wl_registry::Event::Global { name, interface, version } = event {
            let mut logger = LOGGER.try_lock().expect("No se pudo obtener el log");
            writeln!(logger,"name : {}, interface : {}, version : {}", name, interface, version);
            drop(logger);
            if interface == ZKDE_SCREENCAST_UNSTABLE_V1_INTERFACE.name {
                let screencast = proxy.bind::<ZkdeScreencastUnstableV1, _ , _>(name,version.min(1),qhandle,());
                state.screencast_manager = Some(screencast);
            }
        }
        

    }

}


// 2. Manejo de los eventos devueltos por el Stream de captura

impl Dispatch<ZkdeScreencastStreamUnstableV1, ()> for ScreenCastState {

    #![allow(warnings)]
    fn event(
        state: &mut Self,
        proxy: &ZkdeScreencastStreamUnstableV1,
        event: <ZkdeScreencastStreamUnstableV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {

        match event {
            ScreenCastStreamEvents::Created { node } => {},
            ScreenCastStreamEvents::Failed { error } => {},
            ScreenCastStreamEvents::Closed => {},
            ScreenCastStreamEvents::Serial { object_serial_hi, object_serial_low } => {}

        }

    }

}


impl ScreenCastState{
    pub fn new() -> Self{
        Self {
            screencast_manager : None,
            stream: None
        }

    }
} 