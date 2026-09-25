
use zbus::{Connection, zvariant::OwnedFd};

mod utils;

mod dbus;
use dbus::{krunner::WindowsRunnerClient, screencast::ScreenCastRunnerClient};


#[path = "pw-handlers/mod.rs"]
mod pw_handlers;
use pw_handlers::{PipewireHandlerBuilder, StreamEventsHandler, VideoEventsHandlerGPU, VideoEventsHandlerCPU, VideoStreamPipeWireHandler};

use crate::pw_handlers::{StreamPipewireHandler, VideoData};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let connection = Connection::session().await?;
    
    // 1. Listar ventanas disponibles (opcional)...
    let _ :  Result<(), Box<dyn std::error::Error>> = 
    {
        let client = WindowsRunnerClient::new(&connection).await?;

        let windows = client.get_active_windows().await?;
        log_write_async!("--- Ventanas Abiertas ({}) ---", windows.len())?;
        for win in windows {
            log_write_async!("• [{}] {} {} ({} {})", win.id, win.title, win.app_id, win.category, win.relevance)?;
            log_write_async!("  Propiedades[Keys]: {:#?}", win.properties.into_keys().collect::<Vec<_>>())?;
        }
        // logger.flush()?;

        Ok(())
    };

    // 2. Levantar ScreenPortal para seleccionar fuente del streaming...
    let result_screen_portal :  Result<(u32, OwnedFd), Box<dyn std::error::Error>> = {
        let client = ScreenCastRunnerClient::new(&connection).await?;
        // ToDo: Se que hay una forma de hacer que portal_Screen sea opcional, tengo que hacer ingenería inversa
        // para entender como el fichero "restore_data.bin" se construye.
        //let window_uuid = "f630d34f-463a-4743-923c-cb829514f7a8";
        // let mut input: String = String::new();
        // println!("Ingrese el UUID de la ventana a capturar (ej: f630d34f-463a-4743-923c-cb829514f7a8): ");
        // std::io::stdin() 
        //     .read_line(&mut input) 
        //     .expect("Unable to read Stdin");
        
        let result = client.get_pipewire_node_id().await?;

        log_write_async!("--- PipeWire Node ID para la ventana: {} , fd: {:?} ---", result.0, result.1)?;

        Ok(result)
    };

    let (tx , mut rx)  = tokio::sync::mpsc::channel::<VideoData<4>>(1000);
    let events_handler = StreamEventsHandler::<VideoEventsHandlerCPU>::new();
    // 3. Llamar a Pipewire para que empiece a grabar de la fuente y poder empezar a obtener datos...
    let result_streaming: Result<(StreamPipewireHandler, tokio_util::sync::CancellationToken, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> = {
        // Input
        let name = "pipewire-yolo-client";
        let (node, fd) = result_screen_portal?;
        let std_fd : std::os::fd::OwnedFd = fd.into();
        
        // Process
        let mut stream_handler = PipewireHandlerBuilder::new(None)?
            .context(None)?
            .core(std_fd, None)?
            .build()?;
        let _ = stream_handler.default_video_stream(
            name,
            Some(node),
            &events_handler
        )?;
        stream_handler.setup_listener(name,  tx, &events_handler)?;
        let (cancel_token, join_handle) = stream_handler.start();

        // Output
        Ok((stream_handler, cancel_token, join_handle))
    };
 

    //4. Pasamos los frames (normalmente en formato BGRA) a un encoder para transformarlo en un formato consumible (opcional)
    let _ : Result<(), Box<dyn std::error::Error>> = {
        let (_stream_handler, _cancel_token, join_handle) = result_streaming?;
        tokio::pin!(join_handle);
        loop{
            tokio::select! {
                data = rx.recv() => {
                    if let Some(video_data) = data {
                        println!("Dimensiones del video recibido: {:?}", video_data.resolution());
                        println!("Ejemplo de dato: {:?}",  video_data.pixel(400, 800))
                    }
                },
                _ = &mut join_handle => { break; }
            }
        }
       
        Ok(())
    };


    log_write_async!("--- TERMINANDO MAIN ---")?;
    connection.close().await?;
    unsafe{ pipewire::deinit();}

    Ok(())
}

